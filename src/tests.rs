use crate::rpc_server::Blockchain;
use crate::swarms::PubKey;
use crate::test_context::TestContext;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rand::prelude::*;
use rand::seq::SliceRandom;
use std::fmt::{self, Debug};

use futures::future::lazy;

fn sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

#[allow(dead_code)]
pub fn async_test(bc: &Arc<Mutex<Blockchain>>) {
    // let bc = &blockchain;

    let mut ctx = TestContext::new(Arc::clone(&bc));
    ctx.add_swarm(1);

    sleep_ms(300);

    // make a copy here assuming that swarms are not going to change

    let mut rng = StdRng::seed_from_u64(0);
    let ip = bc.lock().unwrap().swarm_manager.swarms[0].nodes[0]
        .ip
        .clone();
    let pk = PubKey::gen_random(&mut rng).to_string();

    let failed_total = Arc::new(Mutex::new(0));
    let saved_total = Arc::new(Mutex::new(0));

    let failed = failed_total.clone();
    let saved = saved_total.clone();

    tokio::run(lazy(move || {
        let client = hyper::Client::new();

        for _ in 0..150 {
            let msg = crate::client::make_random_message(&mut rng);
            let fut = crate::client::send_message_async(&client, &ip, &pk, &msg);

            let mut failed = failed.clone();
            let mut saved = saved.clone();

            let mut failed2 = failed.clone();

            tokio::spawn(
                fut.and_then(move |res| {
                    if res.status() == hyper::StatusCode::OK {
                        *saved.lock().unwrap() += 1;
                    } else {
                        *failed.lock().unwrap() += 1;
                    }
                    Ok(())
                })
                .map_err(move |err| {
                    *failed2.lock().unwrap() += 1;
                    eprintln!("error: {}", err)
                }),
            );
        }

        Ok(())
    }));

    println!("saved total: {}", saved_total.lock().unwrap());
    println!("failed total: {}", failed_total.lock().unwrap());
}

#[allow(dead_code)]
pub fn one_node_big_data(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));
    ctx.add_swarm(1);

    sleep_ms(300);

    // spawn N threads and bombard with messages

    let mut msg_threads = vec![];

    // make a copy here assuming that swarms are not going to change
    let ip = bc.lock().unwrap().swarm_manager.swarms[0].nodes[0]
        .ip
        .clone();

    // I have to manually count the messages as currently I can only use ctx locking
    // in every message (which defeats the purpose of this test)
    let failed_total = Arc::new(Mutex::new(0));
    let saved_total = Arc::new(Mutex::new(0));

    for i in 0..50 {
        let ip = ip.clone();

        let failed_total = failed_total.clone();
        let saved_total = saved_total.clone();

        let t = std::thread::spawn(move || {
            let mut rng = StdRng::seed_from_u64(i);

            let pk = PubKey::gen_random(&mut rng).to_string();

            let mut failed = 0;
            let mut saved = 0;

            for _ in 0..1000 {
                if crate::client::send_message(
                    &ip,
                    &pk,
                    &crate::client::make_random_message(&mut rng),
                )
                .is_ok()
                {
                    saved += 1;
                } else {
                    failed += 1;
                }
            }

            *failed_total.lock().unwrap() += failed;
            *saved_total.lock().unwrap() += saved;
        });

        msg_threads.push(t);
    }

    for t in msg_threads.into_iter() {
        t.join().unwrap();
    }

    sleep_ms(200);

    println!("saved total: {}", saved_total.lock().unwrap());
    println!("failed total: {}", failed_total.lock().unwrap());
}

#[allow(dead_code)]
pub fn long_polling(bc: &Arc<Mutex<Blockchain>>) {
    let mut rng = StdRng::seed_from_u64(0);

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(1);

    sleep_ms(300);

    let pk = PubKey::gen_random(&mut rng);

    let ip = bc.lock().unwrap().swarm_manager.swarms[0].nodes[0]
        .ip
        .clone();
    crate::client::send_message(&ip, &pk.to_string(), "マンゴー").unwrap();

    let ctx_clone = Arc::clone(&ctx);
    let pk_clone = pk.clone();

    std::thread::spawn(move || {
        // check messages every 100 ms
        let mut last_hash = String::new();

        for _ in 0..30 {
            sleep_ms(100);
            let msgs = ctx_clone
                .lock()
                .unwrap()
                .get_new_messages(&pk_clone, &last_hash);
            dbg!(&msgs);

            if !msgs.is_empty() {
                last_hash = msgs.last().unwrap().hash.clone();
            }
        }
    });

    // send another message in 2s
    sleep_ms(2000);
    crate::client::send_message(&ip, &pk.to_string(), "второе сообщение").unwrap();
}

#[allow(dead_code)]
pub fn test_bootstrapping_peer_big_data(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(1);

    sleep_ms(300);

    for _ in 0..10000 {
        ctx.send_random_message();
    }

    ctx.add_snode();
    ctx.inc_block_height();

    sleep_ms(10000);
    ctx.check_messages();
}

#[allow(dead_code)]
pub fn test_bootstrapping_swarm_big_data(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(1);

    sleep_ms(300);

    // TODO: send messages concurrently (asynchronously?)

    for _ in 0..10000 {
        ctx.send_random_message();
    }

    ctx.add_swarm(1);
    ctx.inc_block_height();

    sleep_ms(10000);
    ctx.check_messages();
}

/// 0. Most basic test: send a message to a single snode and check
#[allow(dead_code)]
pub fn single_node_one_message(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(1);

    sleep_ms(300);

    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "マンゴー",
    );

    sleep_ms(2000);

    ctx.check_messages();
}

/// 1. Test that nodes relay messages to other swarm members
#[allow(dead_code)]
pub fn single_swarm_one_message(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(3);

    sleep_ms(300);

    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A2",
    );

    sleep_ms(2000);

    ctx.check_messages();
}

/// 2. Test adding an additional snode to a swarm
#[allow(dead_code)]
pub fn sinlge_swarm_joined(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(3);

    sleep_ms(300);

    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A1",
    );
    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A2",
    );

    sleep_ms(300);

    ctx.add_snode();
    ctx.inc_block_height();

    sleep_ms(3000);

    ctx.check_messages();
}

/// 3. Test new swarm detection
#[allow(dead_code)]
pub fn swarm_splitting(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(3);

    sleep_ms(300);

    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A1",
    );
    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A2",
    );

    sleep_ms(300);

    ctx.add_swarm(3);
    ctx.inc_block_height();

    sleep_ms(2000);

    ctx.check_messages();
}

/// 4. Test multiple swarms with no changes to the swarm composition
#[allow(dead_code)]
pub fn multiple_swarms_static(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(2);
    ctx.add_swarm(2);
    ctx.add_swarm(2);

    sleep_ms(300);

    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A1",
    );
    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A2",
    );

    sleep_ms(2000);

    ctx.check_messages();
}

/// Test that a dissolving swarm will push its data to other swarms
#[allow(dead_code)]
pub fn test_dissolving(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(1);
    ctx.add_swarm(1);
    ctx.add_swarm(1);

    // give SNs some time to initialize their servers
    sleep_ms(300);

    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A",
    );
    ctx.send_message(
        "2b959eac778ee6bfac5e02c29800d489d319b65a9b8960a4cf4d3f40285b7735",
        "B",
    );
    ctx.send_message(
        "18b593e832ffda161c20a5daf842ab787ee7181a369ff7034fe80fb2774e0664",
        "C",
    );
    ctx.send_message(
        "17311f5ae7ce94b79698f12be6f3a2d66ec036fcf77506bf74877381630093af",
        "D",
    );

    sleep_ms(300);

    // Some messages will go to swarm 1, dissolve it
    // Now some of them will go to swarm 0, and the rest will go to swarm 2
    ctx.dissolve_swarm(1);
    ctx.inc_block_height();
    sleep_ms(300);

    ctx.send_message(
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "A2",
    );
    ctx.send_message(
        "2b959eac778ee6bfac5e02c29800d489d319b65a9b8960a4cf4d3f40285b7735",
        "B2",
    );
    ctx.send_message(
        "18b593e832ffda161c20a5daf842ab787ee7181a369ff7034fe80fb2774e0664",
        "C2",
    );
    ctx.send_message(
        "17311f5ae7ce94b79698f12be6f3a2d66ec036fcf77506bf74877381630093af",
        "D2",
    );

    sleep_ms(2000);
    ctx.check_messages();
}

/// 4. Test a node going offline without updating the swarm list
#[allow(dead_code)]
pub fn test_retry_singles(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(2);

    sleep_ms(300);

    // kill a random snode and restore it after 3s
    ctx.restart_snode(3000);

    // Note: sometimes we send a message to a disconnected
    // node, resulting in that message not being accounted
    // for in `check_messages`
    ctx.send_random_message();

    sleep_ms(2000);

    println!("Checking messages (one node offilne, half of the messages should be lost)");
    ctx.check_messages();

    println!("Waiting for a snode to come back online and be tested again...");
    sleep_ms(4000);

    ctx.check_messages();
}

#[allow(dead_code)]
pub fn test_retry_batches(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    // 1. Create a swarm with a single node
    ctx.add_swarm(1);

    // 2. Register a new snode joining the swarm, but
    // don't go online yet.

    ctx.add_snode_delayed(3000);
    ctx.inc_block_height();

    sleep_ms(300);

    // 3. Send a bunch of messages
    ctx.send_random_message();
    ctx.send_random_message();
    ctx.send_random_message();

    sleep_ms(2000);

    println!("Checking messages (one node offilne, half of the messages should be lost)");
    ctx.check_messages();

    println!("Waiting for a snode to come back online and be tested again...");
    sleep_ms(4000);

    ctx.check_messages();
}

fn gen_rand_pubkeys(n: u32, mut rng: &mut StdRng) -> Vec<PubKey> {
    let mut pks = vec![];

    for _ in 0..n {
        let pk = PubKey::gen_random(&mut rng);
        pks.push(pk);
    }

    pks
}

#[derive(Clone)]
pub struct TestOptions {
    pub reliable_snodes: bool,
    pub duration: std::time::Duration,
    pub block_interval: std::time::Duration,
    pub message_interval: std::time::Duration,
}

fn generate_messages_thread(
    ctx: &Arc<Mutex<TestContext>>,
    pks: &Vec<PubKey>,
    opt: TestOptions,
    rng: &StdRng,
    running_flag: &Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    let running = running_flag.clone();
    let mut rng_clone = rng.clone();
    let ctx = ctx.clone();
    let pks = pks.clone();
    // Spawn a thread for messages
    std::thread::spawn(move || {
        for _ in 0.. {
            // give SNs some time to initialize their servers
            std::thread::sleep(opt.message_interval);

            let pk = pks.choose(&mut rng_clone).unwrap();

            ctx.lock()
                .unwrap()
                .send_random_message_to_pk(&pk.to_string());

            if !running.load(Ordering::SeqCst) {
                break;
            }
        }
    })
}

// Should this be a separate thread?
fn generate_blocks(
    ctx: &Arc<Mutex<TestContext>>,
    opt: TestOptions,
    rng: &mut StdRng,
    running_flag: &Arc<AtomicBool>,
) {
    // Every iteration in this loop corresponds to a block
    for i in 0.. {
        println!("iteration: {}", i);
        info!("iteration: {}", i);
        // how much to wait until the next block
        // let ms = rng.gen_range(500, 2000);
        std::thread::sleep(opt.block_interval);

        let mut ctx = ctx.lock().unwrap();

        // deregister some
        if ctx.snode_count() > 10 {
            let n = rng.gen_range(0, 3);
            for _ in 0..n {
                ctx.drop_snode();
            }

            // IMPORTANT: it seems that setting this to a long delay
            // prevents SN from recovering. Could be because they miss
            // a few block updates
            if !opt.reliable_snodes {
                ctx.restart_snode(1000);
            }
        }

        // register some
        if ctx.snode_count() < 50 {
            let n = rng.gen_range(0, 3);
            for _ in 0..n {
                ctx.add_snode();
            }
        }

        ctx.inc_block_height();

        println!("swarms: {:?}", *ctx);

        // release the lock
        drop(ctx);

        if !running_flag.load(Ordering::SeqCst) {
            break;
        }
    }
}

/// `reliable` determines whether nodes can disconnect from time
/// to time for a short period of time
#[allow(dead_code)]
pub fn test_blocks(bc: &Arc<Mutex<Blockchain>>, opt: &TestOptions) {
    let mut rng = StdRng::seed_from_u64(0);

    let pks = gen_rand_pubkeys(100, &mut rng);

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(3);

    // PART I
    {
        let running_flag = Arc::new(AtomicBool::new(true));

        let running = running_flag.clone();

        let duration = opt.duration;
        // TODO: use a timer instead
        let timer_thread = std::thread::spawn(move || {
            std::thread::sleep(duration);
            running.store(false, Ordering::SeqCst);
        });

        let message_thread = generate_messages_thread(&ctx, &pks, opt.clone(), &rng, &running_flag);

        generate_blocks(&ctx, opt.clone(), &mut rng, &running_flag);

        timer_thread.join().unwrap();
        message_thread.join().unwrap();

        // wait for the duration of one block to
        // make sure all message have been propagated
        std::thread::sleep(opt.block_interval);

        ctx.lock().unwrap().print_stats();
        ctx.lock().unwrap().check_messages();
    }


    // // PART II
    // {
    //     let running_flag = Arc::new(AtomicBool::new(true));

    //     let running = running_flag.clone();

    //     let duration = opt.duration;
    //     // TODO: use a timer instead
    //     let timer_thread = std::thread::spawn(move || {
    //         std::thread::sleep(duration);
    //         running.store(false, Ordering::SeqCst);
    //     });

    //     let message_thread = generate_messages_thread(&ctx, &pks, opt.clone(), &rng, &running_flag);

    //     generate_blocks(&ctx, opt.clone(), &mut rng, &running_flag);

    //     timer_thread.join().unwrap();
    //     message_thread.join().unwrap();

    //     // wait for the duration of one block to
    //     // make sure all message have been propagated
    //     std::thread::sleep(opt.block_interval);

    //     ctx.lock().unwrap().print_stats();
    //     ctx.lock().unwrap().check_messages();
    // }


}

#[allow(dead_code)]
pub fn test_with_wierd_clients(bc: &Arc<Mutex<Blockchain>>) {
    let sm = &mut bc.lock().unwrap().swarm_manager;

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    let bc = Arc::clone(&bc);

    sm.add_swarm(&[1]);

    std::thread::spawn(move || {
        // give SNs some time to initialize their servers
        sleep_ms(200);

        // Construct an unreasonably large message:
        let mut large_msg = String::new();

        // NOTE: Our server fails on this (Error(9): body limit exceeded)
        for _ in 0..200000 {
            large_msg.push_str("012345657");
        }

        ctx.lock().unwrap().send_message(
            "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
            &large_msg,
        );

        sleep_ms(2000);

        ctx.lock().unwrap().check_messages();

        sleep_ms(2000);

        ctx.lock().unwrap().check_messages();
    });
}

use tokio::prelude::*;

#[allow(dead_code)]
fn test_tokio() {
    let fut = tokio::timer::Delay::new(
        std::time::Instant::now() + std::time::Duration::from_millis(1000),
    );

    let fut = fut
        .and_then(|_| {
            println!("time out!");
            Ok(())
        })
        .map_err(|_e| panic!("timer failed"));

    tokio::run(fut);
}

#[allow(dead_code)]
fn test_small_random(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(3);

    sleep_ms(1000);
    ctx.add_snode();

    ctx.send_random_message();
    sleep_ms(100);
    ctx.send_random_message();
    sleep_ms(100);
    ctx.send_random_message();
    sleep_ms(100);

    ctx.drop_snode();

    ctx.add_snode();
    ctx.add_snode();
    ctx.add_snode();

    sleep_ms(2000);

    ctx.check_messages();
}

/// 2. Test adding an additional snode to a swarm
#[allow(dead_code)]
pub fn peer_testing(bc: &Arc<Mutex<Blockchain>>) {
    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(5);

    sleep_ms(300);

    for _ in 0..10 {
        ctx.send_random_message();
    }

    sleep_ms(300);

    for _ in 0..3 {
        // new block every 1s
        sleep_ms(1000);
        ctx.inc_block_height();
    }

    sleep_ms(3000);

    ctx.check_messages();
}