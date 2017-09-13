Detail
======

There's a README in the `submission/` directory that talks a little about
the design of the program.

When doing code review, you can examine anything in the `submission/`
directory except `script.sh` (which contains exploit details), e.g. the
Dockerfile is clean, but feel free to check it. For vulnerability spotting:

 - Assume that if a request is made, it's been authenticated (e.g. with a
   password, card number) beforehand - there's no checking of deposit amounts,
   or passwords for logging into accounts and that's fine.

 - Panicking (typically via unwrap) can be assumed to be irrelevant and
   is only there to reduce the noise of error handling.

 - There's no persistence of accounts. Pretend the server will run forever.

 - Assume the fraud checker is a black box that makes some requests to
   external services - the details aren't important.

Verifying the exploit
---------------------

Ideally you just go into the `submission/` directory and run `docker build .`.
This will build the Dockerfile and run the tests, then you can run the built
image to execute `./script.sh` and verify the exploit.

It should look something like this:

```
$ docker build --tag salami .
running 1 test
test currency::test_lookup ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured

 ---> c76506535db7
Removing intermediate container 3d2f9539fad2
Step 8/8 : CMD ./target/debug/underhanded-rs & sleep 5 && ./script.sh && exit
 ---> Running in 0992c96a4a61
 ---> 1036b909999e
Removing intermediate container 0992c96a4a61
Successfully built 1036b909999e
$ docker run -it salami
Server starting on port 3000
Deposited 500
Working
acct 4 has balance 501
```

The script deposited 500 and ended up with an account with 501 in it.

If you don't have Docker, you'll need bash, curl and a fairly normal Linux.
Just `cargo run` and run `./script.sh`. If `script.sh` thinks it's going to
have problems it'll let you know.
