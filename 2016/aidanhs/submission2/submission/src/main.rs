extern crate iron;
extern crate router;

extern crate crossbeam;
#[macro_use]
extern crate lazy_static;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::sync::RwLock;
use std::time::Instant;

use iron::prelude::Iron;

use router::Router;

#[derive(Debug)]
pub struct UserAccount {
    /// Balance in the base units of the currency, e.g. cents
    balance: i64,
    /// Currency for this account
    currency: &'static currency::CurrencyDetail,
    /// Times of 'recent' transfers
    recent_transfers: Vec<Instant>,
}
impl UserAccount {
    fn new(currency: &str) -> Option<UserAccount> {
        if let Some(cur) = currency::lookup_currency(currency) {
            Some(UserAccount {
                currency: cur,
                balance: 0,
                recent_transfers: vec![],
            })
        } else {
            None
        }
    }
    fn fakeacct() -> UserAccount {
        UserAccount {
            currency: &currency::FAKE_CURRENCY,
            balance: 0,
            recent_transfers: vec![],
        }
    }
}

struct UserDB {
    accts: Vec<UserAccount>,
}
impl UserDB {
    fn addacct(&mut self, acct: UserAccount) -> usize {
        let id = self.accts.len();
        self.accts.push(acct);
        id
    }
    fn get(&self, id: usize) -> Option<&UserAccount> {
        self.accts.get(id)
    }
    fn get_mut(&mut self, id: usize) -> Option<&mut UserAccount> {
        self.accts.get_mut(id)
    }
    fn get2_mut(&mut self, id1: usize, id2: usize) -> (Option<&mut UserAccount>, Option<&mut UserAccount>) {
        if id1 > id2 {
            let (as1, as2) = self.accts.split_at_mut(id1);
            (as2.get_mut(0), as1.get_mut(id2))
        } else if id1 < id2 {
            let (as1, as2) = self.accts.split_at_mut(id2);
            (as1.get_mut(id1), as2.get_mut(0))
        } else {
            panic!()
        }
    }
}

const FAKE_CHARGE_ACCT: usize = 0;
const USD_CHARGE_ACCT:  usize = 1;
const EUR_CHARGE_ACCT:  usize = 2;
const GBP_CHARGE_ACCT:  usize = 3;
lazy_static! {
    // Separate by currency for tax reasons
    static ref USERDB: RwLock<UserDB> = RwLock::new(UserDB { accts: vec![
        UserAccount::fakeacct(),
        UserAccount::new("USD").unwrap(),
        UserAccount::new("EUR").unwrap(),
        UserAccount::new("GBP").unwrap(),
    ]});
}

fn main() {
    let mut router = Router::new();
    router.get("/dumpbalance", routes::dumpbalance, "dumpbalance"); // debug
    router.post("/makeaccount", routes::makeaccount_handler, "makeaccount");
    router.post("/deposit", routes::deposit_handler, "deposit");
    router.post("/transfer", routes::transfer_handler, "transfer");

    println!("Server starting on port 3000");
    Iron::new(router).http("0.0.0.0:3000").unwrap();
}

mod routes {
    use std::i64;
    use std::time::{Duration, Instant};
    use std::thread;

    use crossbeam;

    use iron::prelude::{IronResult, Request, Response};
    use iron::status;

    use serde_json;

    use super::USERDB;
    use super::UserAccount;
    use super::currency;
    use super::fraud::fraud_checker;

    // Consider requests in the last `RATE_LIMIT_SECS` secs to contribute
    // towards rate limiting
    const RATE_LIMIT_SECS: u64 = 60;

    // Each request in the last `RATE_LIMIT_SECS` contributes `MS_PER_REQ_RATE`
    // milliseconds to the delay to fulfil the request
    const MS_PER_REQ_RATE: usize = 50;

    macro_rules! resp {
        ($status:ident, $msg:expr) => {{
            Ok(Response::with((status::$status, $msg)))
        }};
    }

    #[derive(Deserialize)]
    struct DumpBalance {
        account_id: usize,
    }
    pub fn dumpbalance(req: &mut Request) -> IronResult<Response> {
        let obj: DumpBalance = serde_json::from_reader(&mut req.body).unwrap();
        let userdb = USERDB.read().unwrap();
        resp!(Ok, format!("acct {} has balance {}\n",
            obj.account_id, userdb.get(obj.account_id).unwrap().balance))
    }

    #[derive(Deserialize)]
    struct MakeAccount {
        currency: String,
    }
    /// Create account `account_name` with the currency set to `currency`
    pub fn makeaccount_handler(req: &mut Request) -> IronResult<Response> {
        let obj: MakeAccount = serde_json::from_reader(&mut req.body).unwrap();
        let mut userdb = USERDB.write().unwrap();
        if let Some(acct) = UserAccount::new(&obj.currency) {
            resp!(Ok, userdb.addacct(acct).to_string())
        } else {
            resp!(BadRequest, "invalid currency specified")
        }
    }

    #[derive(Deserialize)]
    struct Deposit {
        account_id: usize,
        amount: u64,
    }
    /// Deposit `amount` of the base unit of the currency for the specified
    /// `account_id` into that account's balance
    pub fn deposit_handler(req: &mut Request) -> IronResult<Response> {
        let obj: Deposit = serde_json::from_reader(&mut req.body).unwrap();
        assert!(obj.amount < i64::MAX as u64);
        let amount = obj.amount as i64;
        match USERDB.write().unwrap().get_mut(obj.account_id) {
            Some(ua) => ua.balance += amount,
            None => return resp!(BadRequest, "user does not exist"),
        }
        resp!(Ok, "")
    }

    #[derive(Deserialize)]
    struct Transfer {
        account_from: usize,
        account_to: usize,
        amount: u64,
    }
    /// Transfer `amount` from `account_from` to `account_to`
    pub fn transfer_handler(req: &mut Request) -> IronResult<Response> {
        let obj: Transfer = serde_json::from_reader(&mut req.body).unwrap();
        assert!(obj.amount < i64::MAX as u64);
        let amount = obj.amount as i64;
        if amount < currency::MINIMUM_TRANSFER_AMOUNT {
            return resp!(BadRequest, "below minimum transfer")
        }

        let mut userdb = USERDB.write().unwrap();
        let mut currency_detail = None;
        // Check the transfer is valid
        {
        let (acct_from, acct_to) = match userdb.get2_mut(obj.account_from, obj.account_to) {
            (Some(acct_from), Some(acct_to)) => (acct_from, acct_to),
            _ => return resp!(BadRequest, "one or both accounts do not exist"),
        };
        let maybe_err = crossbeam::scope(|scope| {
            // The request isn't totally wrong, so filter old requests, note down that
            // a request has been attempted (to contribute towards rate limiting) and
            // calculate how much the user should be rate limited right now (
            let rate_limit_bound = Duration::new(RATE_LIMIT_SECS, 0);
            acct_from.recent_transfers.retain(|inst| inst.elapsed() < rate_limit_bound);
            acct_from.recent_transfers.push(Instant::now());
            let wait_millis = acct_from.recent_transfers.len() * MS_PER_REQ_RATE;
            // Cap at a 4s wait to not overflow u32
            let wait_millis = if wait_millis > 4000 { 4000 } else { wait_millis } as u32;
            let rate_limit_wait = Duration::new(0, wait_millis * 1_000_000);

            if acct_from.balance + (acct_from.currency.overdraft_limit as i64) < amount {
                return Some(resp!(BadRequest, "balance too low in account_from"))
            }
            if acct_from.currency != acct_to.currency {
                return Some(resp!(BadRequest, "user account currencies do not match"))
            }
            let err_spawns = vec![
                scope.spawn(|| fraud_checker().check(acct_from)),
                scope.spawn(|| fraud_checker().check(acct_to)),
            ];
            // Do the rate limit in parallel with fraud checking, no need for the user to
            // have to wait twice
            thread::sleep(rate_limit_wait);
            // Save the currency details
            currency_detail = Some(&*acct_from.currency);
            // Retrieve the fraud check results
            for spawn in err_spawns {
                if let Some(_fraud_err) = spawn.join() {
                    return Some(resp!(BadRequest, "request rejected, possible fraud"))
                }
            }
            None
        });
        if let Some(err) = maybe_err {
            return err
        }
        }

        // Transfer is validated, let's go!
        let currency_detail = currency_detail.unwrap();
        let charge_amount = f64::ceil(currency_detail.transfer_charge/100.0 * amount as f64) as i64;
        let charge_account = currency_detail.transfer_charge_account;

        userdb.get_mut(obj.account_from).unwrap().balance -= amount + charge_amount;
        userdb.get_mut(obj.account_to).unwrap().balance += amount;
        userdb.get_mut(charge_account).unwrap().balance += charge_amount;
        resp!(Ok, "")
    }
}

mod fraud {
    use super::UserAccount;

    use std::mem;
    use std::sync::Mutex;
    use std::sync::mpsc::SyncSender;
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;
    use std::thread;

    type FraudCheckResult = (UserAccount, Option<FraudError>);
    type FraudCheckRequest = (SyncSender<FraudCheckResult>, UserAccount);

    #[derive(Clone)]
    pub struct FraudChecker {
        tx: SyncSender<FraudCheckRequest>,
    }
    impl FraudChecker {
        pub fn check(&self, acct: &mut UserAccount) -> Option<FraudError> {
            let (tx, rx) = sync_channel(0);
            self.tx.send((tx, mem::replace(acct, UserAccount::fakeacct()))).unwrap();
            let (checked_acct, err) = rx.recv().unwrap();
            *acct = checked_acct;
            err
        }
    }

    pub struct FraudError(String);

    // Start the fraud checker on first use
    lazy_static! {
        static ref FRAUD_CHECKER: Mutex<FraudChecker> = Mutex::new(start_checker());
    }

    pub fn start_checker() -> FraudChecker {
        let (tx, rx) = sync_channel::<FraudCheckRequest>(0);
        thread::spawn(move || {
            while let Ok((tx, acct)) = rx.recv() {
                // TODO: query external fraud checking service, perform our own analysis
                // and send back the summary
                thread::sleep(Duration::new(0, 500_000_000)); // 0.5s placeholder delay
                tx.send((acct, None)).unwrap()
            }
        });
        FraudChecker { tx: tx }
    }

    pub fn fraud_checker() -> FraudChecker {
        FRAUD_CHECKER.lock().unwrap().clone()
    }
}

mod currency {
    use super::{FAKE_CHARGE_ACCT, EUR_CHARGE_ACCT, GBP_CHARGE_ACCT, USD_CHARGE_ACCT};

    /// Minimum base units of currency permitted to be transferred
    pub const MINIMUM_TRANSFER_AMOUNT: i64 = 50;

    /// Global table of currency details
    static CURRENCIES: &'static [CurrencyDetail] = &[
        CurrencyDetail {
            name: "EUR",
            overdraft_limit: 10,
            transfer_charge: 1.0,
            transfer_charge_account: EUR_CHARGE_ACCT,
        },
        CurrencyDetail {
            name: "GBP",
            overdraft_limit: 10,
            transfer_charge: 1.0,
            transfer_charge_account: GBP_CHARGE_ACCT,
        },
        CurrencyDetail {
            name: "USD",
            overdraft_limit: 5,
            transfer_charge: 2.0,
            transfer_charge_account: USD_CHARGE_ACCT,
        },
    ];

    /// Sentinel value for currency, api users cannot touch this
    pub static FAKE_CURRENCY: CurrencyDetail = CurrencyDetail {
        name: "FAKE",
        overdraft_limit: 0,
        transfer_charge: -1.0,
        transfer_charge_account: FAKE_CHARGE_ACCT,
    };


    #[derive(Debug)]
    pub struct CurrencyDetail {
        /// Name used to identify the currency
        pub name: &'static str,
        /// Overdraft limit for this currency
        pub overdraft_limit: u64,
        /// Percentage charge when transferring money
        pub transfer_charge: f64,
        /// Account to deposit transfer charges into
        pub transfer_charge_account: usize,
    }

    /// Currencies are always static, so just compare the pointers
    impl PartialEq for CurrencyDetail {
        fn eq(&self, other: &Self) -> bool {
            self as *const _ == other as *const _
        }
    }
    impl Eq for CurrencyDetail {}

    pub fn lookup_currency(currency_name: &str) -> Option<&'static CurrencyDetail> {
        CURRENCIES.iter().find(|cur| currency_name == cur.name)
    }

    #[test]
    fn test_lookup() {
        assert!(lookup_currency("EUR").is_some());
        assert!(lookup_currency("FAKE").is_none());
    }
}
