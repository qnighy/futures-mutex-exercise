#![feature(async_await, await_macro, pin, futures_api)]

use std::rc::Rc;

use futures::prelude::*;
use rand::prelude::*;

use tokio::runtime::current_thread::{Runtime, spawn};
use futures_mutex::unsync::Mutex;
use futures_test::future::FutureTestExt;

async fn jitter() {
    let num = thread_rng().gen_range(0, 10);
    for _ in 0..num {
        await!(async {}.pending_once());
    }
}

async fn main2() {
    let resources = (0..5_i32).map(|i| Rc::new(Mutex::new(i))).collect::<Vec<_>>();
    for i in 0..5 {
        let (res0, res1) = if i == 4 {
            (resources[0].clone(), resources[4].clone())
        } else {
            (resources[i].clone(), resources[i + 1].clone())
        };
        spawn(async move {
            for _ in 0..100 {
                let lock0 = await!(res0.lock()).unwrap();
                await!(jitter());
                eprintln!("Thread {}: acquired {}", i, *lock0);

                let lock1 = await!(res1.lock()).unwrap();
                await!(jitter());
                eprintln!("Thread {}: acquired {}", i, *lock1);

                drop(lock1);
                await!(jitter());
                drop(lock0);
                await!(jitter());
            }
            println!("Thread {}: done!", i);
            Ok(())
        }.boxed().compat());
    }
}

fn main() {
    let mut rt = Runtime::new().unwrap();
    rt.spawn(async {
        await!(main2());
        Ok(())
    }.boxed().compat());
    rt.run().unwrap();
}
