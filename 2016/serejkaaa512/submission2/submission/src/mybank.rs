use rustc_serialize::json;
use rand;

#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct Person {
    pub firstname: String,
    pub lastname: String,
}

#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct Payment {
    pub customer: Person,
    pub account: String,
    pub amount: f64,
}

#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct AccountInfo {
    pub customer: Person,
    pub account: String,
    pub amount: f64,
}

#[derive(Debug)]
pub struct Bank {
    accounts: Vec<AccountInfo>,
}

pub trait Service {
    fn add_payment(&mut self, pay: Payment) -> String;
    fn add_customer(&mut self, customer: Person) -> String;
    fn get_account_info(&mut self, customer: Person) -> String;
}


impl Bank {
    pub fn new() -> Bank {
        Bank { accounts: vec![] }
    }
}

impl Service for Bank {
    fn add_customer(&mut self, customer: Person) -> String {
        let account = rand::random::<i32>().abs().to_string();
        let amount = 0f64;
        let ac = AccountInfo {
            customer: customer,
            account: account.clone(),
            amount: amount,
        };
        self.accounts.push(ac.clone());
        json::encode(&ac).unwrap()
    }

    fn add_payment(&mut self, pay: Payment) -> String {
        let ac = self.accounts
            .iter_mut()
            .find(|ac| {
                ac.customer.firstname == pay.customer.firstname &&
                ac.customer.lastname == pay.customer.lastname &&
                ac.account == pay.account
            })
            .unwrap();
        ac.amount += pay.amount;
        json::encode(ac).unwrap()
    }

    fn get_account_info(&mut self, customer: Person) -> String {
        let ac = self.accounts
            .iter()
            .find(|ac| {
                ac.customer.firstname == customer.firstname &&
                ac.customer.lastname == customer.lastname
            })
            .unwrap();
        json::encode(&ac).unwrap()
    }
}
