extern crate iron;
extern crate router;

#[macro_use]
extern crate lazy_static;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use iron::prelude::Iron;

use router::Router;

#[derive(Debug)]
struct UserAccount {
    /// Canonical currency name - sanitised before inserting
    currency: String,
    /// Balance in the base units of the currency, e.g. cents
    balance: u64,
    /// Has account been disabled
    disabled: bool,
}
impl UserAccount {
    fn new(currency: &str) -> UserAccount {
        UserAccount {
            currency: currency.to_owned(),
            balance: 0,
            disabled: false,
        }
    }
}

lazy_static! {
    static ref USERDB: RwLock<HashMap<String, UserAccount>> = {
        let mut m = HashMap::new();
        // Separate by currency for tax reasons
        m.insert("usd_transfers1".to_owned(), UserAccount::new("USD"));
        m.insert("usd_transfers2".to_owned(), UserAccount::new("USD"));
        m.insert("eur_transfers".to_owned(), UserAccount::new("EUR"));
        m.insert("gbp_transfers".to_owned(), UserAccount::new("GBP"));
        RwLock::new(m)
    };
}

fn main() {
    {
        let userdb = USERDB.read().unwrap();
        let known_currencies: HashSet<_> = userdb.values()
            .map(|ua| &ua.currency).collect();
        for currency in known_currencies {
            println!("Loading currency: {}", currency);
            currency::load_currency(currency).unwrap();
        }
    }

    let mut router = Router::new();
    router.get("/dumpbalance", routes::dumpbalance, "dumpbalance"); // debug
    router.post("/makeaccount", routes::makeaccount_handler, "makeaccount");
    router.post("/deposit", routes::deposit_handler, "deposit");
    router.post("/transfer", routes::transfer_handler, "transfer");

    println!("Server starting on port 3000");
    Iron::new(router).http("0.0.0.0:3000").unwrap();
}

mod routes {
    use iron::prelude::{IronResult, Request, Response};
    use iron::status;

    use serde_json;

    use super::USERDB;
    use super::UserAccount;
    use super::currency;

    macro_rules! resp {
        ($status:ident, $msg:expr) => {{
            Ok(Response::with((status::$status, $msg)))
        }};
    }

    #[derive(Deserialize)]
    struct DumpBalance {
        account_name: String,
    }
    pub fn dumpbalance(req: &mut Request) -> IronResult<Response> {
        let obj: DumpBalance = serde_json::from_reader(&mut req.body).unwrap();
        let userdb = USERDB.read().unwrap();
        resp!(Ok, format!("acct {} has balance {}\n",
            obj.account_name, userdb.get(&obj.account_name).unwrap().balance))
    }

    #[derive(Deserialize)]
    struct MakeAccount {
        account_name: String,
        currency: String,
    }
    /// Create account `account_name` with the currency set to `currency`
    pub fn makeaccount_handler(req: &mut Request) -> IronResult<Response> {
        let obj: MakeAccount = serde_json::from_reader(&mut req.body).unwrap();
        let currency = match currency::lookup_currency(&obj.currency) {
            Ok(currency_detail) => currency_detail.canonical_name,
            Err(_) => return resp!(BadRequest, "unknown currency"),
        };
        let mut userdb = USERDB.write().unwrap();
        if userdb.contains_key(&obj.account_name) {
            return resp!(BadRequest, "user exists")
        }
        userdb.insert(obj.account_name, UserAccount::new(&currency));
        resp!(Ok, "")
    }

    #[derive(Deserialize)]
    struct Deposit {
        account_name: String,
        amount: u64,
    }
    /// Deposit `amount` of the base unit of the currency for the specified
    /// `account_name` into that account's balance
    pub fn deposit_handler(req: &mut Request) -> IronResult<Response> {
        let obj: Deposit = serde_json::from_reader(&mut req.body).unwrap();
        match USERDB.write().unwrap().get_mut(&obj.account_name) {
            Some(ua) => ua.balance += obj.amount,
            None => return resp!(BadRequest, "user does not exist"),
        }
        resp!(Ok, "")
    }

    #[derive(Deserialize)]
    struct Transfer {
        account_from: String,
        account_to: String,
        amount: u64,
    }
    /// Transfer `amount` from `account_from` to `account_to`
    pub fn transfer_handler(req: &mut Request) -> IronResult<Response> {
        let obj: Transfer = serde_json::from_reader(&mut req.body).unwrap();
        let amount = obj.amount;
        if amount < currency::MINIMUM_TRANSFER_AMOUNT {
            return resp!(BadRequest, "below minimum transfer")
        }
        let currency;
        let mut userdb = USERDB.write().unwrap();
        match (userdb.get(&obj.account_from), userdb.get(&obj.account_to)) {
            (Some(uaf), _) if uaf.balance < amount =>
                return resp!(BadRequest, "balance too low in account_from"),
            (Some(uaf), Some(uat)) if uaf.currency != uat.currency =>
                return resp!(BadRequest, "user account currencies do not match"),
            (Some(uaf), Some(_)) => currency = uaf.currency.clone(), // valid request
            _ => return resp!(BadRequest, "one or both accounts do not exist"),
        }

        let currency_detail = currency::lookup_currency(&currency).unwrap(); // currency from db is already sanitised
        let charge_amount = f64::ceil(currency_detail.transfer_charge/100.0 * amount as f64) as u64;
        let charge_account = currency_detail.transfer_charge_accounts.iter()
            .find(|&ua| !userdb.get(ua).unwrap().disabled).unwrap();

        userdb.get_mut(&obj.account_from).unwrap().balance -= amount + charge_amount;
        userdb.get_mut(&obj.account_to).unwrap().balance += amount;
        userdb.get_mut(charge_account).unwrap().balance += charge_amount;
        resp!(Ok, "")
    }
}

mod currency {
    use serde_json;

    use std::collections::HashMap;
    use std::env::current_dir;
    use std::fs::File;
    use std::io;
    use std::path::PathBuf;
    use std::sync::RwLock;

    /// If not overriden by a currency json, the default transfer charge
    const DEFAULT_TRANSFER_CHARGE: f64 = 1.0;

    /// Minimum base units of currency permitted to be transferred
    pub const MINIMUM_TRANSFER_AMOUNT: u64 = 50;

    pub static CURRENCY_SUBDIR: &'static str = "currency";

    lazy_static! {
        /// Path containing all currency json files
        static ref CURRENCY_DATA_DIR: PathBuf =
            current_dir().unwrap().join(CURRENCY_SUBDIR);

        /// Global table of currency details
        static ref CURRENCIES: RwLock<HashMap<String, CurrencyDetail>> =
            RwLock::new(HashMap::new());
    }

    #[derive(Deserialize, Debug)]
    struct Currency {
        /// Natively used name for display purposes, e.g. 'dollars', 'euros'
        name: String,
        /// Countries using the currency, for UI suggestion purposes
        countries: Option<Vec<String>>,
        /// Additional Quadrilateral-specific information
        features: CurrencyFeatures,
    }

    #[derive(Deserialize, Debug)]
    struct CurrencyFeatures {
        /// Acceptable aliases for this currency in transfer requests
        aliases: Option<Vec<String>>,
        /// See CurrencyDetail
        transfer_charge: Option<f64>,
        /// See CurrencyDetail
        transfer_charge_accounts: Vec<String>,
    }

    #[derive(Clone, Debug)]
    pub struct CurrencyDetail {
        /// Canonical name of the currency for loading it
        pub canonical_name: String,
        /// Percentage charge when transferring money, or default of 1%
        pub transfer_charge: f64,
        /// Accounts to attempt to deposit transfer charges into in order,
        /// using next one if disabled (allowing account rotation when
        /// doing taxes etc)
        pub transfer_charge_accounts: Vec<String>,
    }

    #[derive(Debug)]
    pub struct CurrencyError(&'static str);
    impl Default for CurrencyError {
        fn default() -> CurrencyError {
            CurrencyError("invalid currency")
        }
    }
    impl From<io::Error> for CurrencyError {
        fn from(_: io::Error) -> CurrencyError {
            Default::default()
        }
    }
    impl From<serde_json::Error> for CurrencyError {
        fn from(_: serde_json::Error) -> CurrencyError {
            CurrencyError("invalid currency")
        }
    }

    pub fn lookup_currency(currency_name: &str) -> Result<CurrencyDetail, CurrencyError> {
        let exists = CURRENCIES.read().unwrap().contains_key(currency_name);
        if !exists {
            // Load currencies dynamically if possible, so we don't have to restart
            // whenever a new currency is added to the filesystem
            load_currency(currency_name)?;
        }
        Ok(CURRENCIES.read().unwrap().get(currency_name).unwrap().clone())
    }

    /// Attempt to load a currency into the API server, using json files
    /// from the currency data directory
    pub fn load_currency(currency_id: &str) -> Result<(), CurrencyError> {
        let currency_path = CURRENCY_DATA_DIR.join(currency_id);
        if !currency_path.is_file() { return Err(Default::default()) }
        // Check for .. or symlinks which could put us outside the data dir
        let canon_currency_path = currency_path.canonicalize()?;
        if currency_path != canon_currency_path {
            return Err(Default::default())
        }
        let currency_file = File::open(currency_path)?;
        let currency: Currency = serde_json::from_reader(currency_file)?;
        let transfer_charge = currency.features.transfer_charge
            .unwrap_or(DEFAULT_TRANSFER_CHARGE);
        let currency_detail = CurrencyDetail {
            canonical_name: currency_id.to_owned(),
            transfer_charge: transfer_charge,
            transfer_charge_accounts: currency.features.transfer_charge_accounts,
        };

        // Populate the currency id and all aliases with currency info
        let mut currencies = CURRENCIES.write().unwrap();
        currencies.insert(currency_id.to_owned(), currency_detail.clone());
        if let Some(aliases) = currency.features.aliases {
            for alias in aliases {
                currencies.insert(alias, currency_detail.clone());
            }
        }
        Ok(())
    }

    #[test]
    fn check_dir_traversal() {
        fn check_case(cur: &str, is_ok: bool) {
            println!("Checking {}", cur);
            assert!(CURRENCY_DATA_DIR.join(cur).is_file());
            assert!(load_currency(cur).is_ok() == is_ok);
        }
        check_case("EUR", true);
        check_case(&format!("../{}/EUR", CURRENCY_SUBDIR), false);
    }
}
