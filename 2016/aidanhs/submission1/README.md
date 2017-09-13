Detail
======

There's a README in the `submission/` directory that talks a little about
the design of the program.

When doing code review, you can examine anything in the `submission/`
directory except `script.sh` (which contains exploit details), e.g. the
Dockerfile is clean, but feel free to check it.

The code itself is intended to be very minimal as it's more fun to hide
issues in less code. As a result, a few sacrifices of functionality
has been made in the name of simplicity and conciseness:

 - Assume that if a request is made, it's been authenticated (e.g. with a
   password, card number) beforehand - there's no checking of deposit amounts,
   or passwords for logging into accounts and that's fine.

 - Panicking (typically via unwrap) can be assumed to be irrelevant and
   is only there to reduce the noise of error handling.

 - There's no persistence of accounts. Pretend the server will run forever.

One final note specific to this exploit - if you think you've found something,
I encourage you to continue and try and construct the full story.

Verifying the exploit
---------------------

Ideally you just go into the `submission/` directory and run `docker build .`.
This will build the Dockerfile and run the tests, then you can run the built
image to execute `./script.sh` and verify the exploit.

It should look something like this:

```
$ docker build --tag salami .
[...]
running 1 test
test currency::check_dir_traversal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured

 ---> bcd0ddf9ef05
Removing intermediate container 64fbe06a592d
Step 8/8 : CMD ./target/debug/underhanded-rs & sleep 5 && ./script.sh && exit
 ---> Running in 58a912f117c8
 ---> 30a83fc5c5e7
Removing intermediate container 58a912f117c8
Successfully built 30a83fc5c5e7
$ docker run -it --rm salami
Loading currency: GBP
Loading currency: EUR
Loading currency: USD
Server starting on port 3000
acct aidanhs has balance 0
acct aidanhs has balance 1
```

The two `acct aidanhs` lines demonstrate the salami being sliced.

If you don't have Docker, you'll need bash, curl and a fairly normal Linux.
Just `cargo run` and run `./script.sh`. If `script.sh` thinks it's going to
have problems it'll let you know.
