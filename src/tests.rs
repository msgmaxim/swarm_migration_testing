
use std::sync::{Mutex, Arc};
use crate::test_context::TestContext;
use crate::rpc_server::Blockchain;
use crate::swarms::PubKey;


use rand::seq::SliceRandom;
use rand::prelude::*;
use std::fmt::{self, Debug};

/// 0. Most basic test: send a message to a single snode and check
pub fn single_node_one_message(bc: Arc<Mutex<Blockchain>>) {

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(1);

    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A2");

    std::thread::sleep(std::time::Duration::from_millis(2000));

    ctx.lock().unwrap().check_messages();
}

/// 1. Test that nodes relay messages to other swarm members
pub fn single_swarm_one_message(bc: Arc<Mutex<Blockchain>>) {

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(3);

    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A2");

    std::thread::sleep(std::time::Duration::from_millis(2000));

    ctx.lock().unwrap().check_messages();
}

/// 2. Test adding an additional snode to a swarm
pub fn sinlge_swarm_joined(bc: Arc<Mutex<Blockchain>>) {

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(3);

    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A1");
    ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A2");

    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.lock().unwrap().add_snode();

    std::thread::sleep(std::time::Duration::from_millis(2000));

    ctx.lock().unwrap().check_messages();
}

/// 3. Test new swarm detection
pub fn swarm_splitting(bc: Arc<Mutex<Blockchain>>) {

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(3);

    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A1");
    ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A2");

    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.lock().unwrap().add_swarm(3);

    std::thread::sleep(std::time::Duration::from_millis(2000));

    ctx.lock().unwrap().check_messages();
}

/// 4. Test multiple swarms with no changes to the swarm composition
pub fn multiple_swarms_static(bc: Arc<Mutex<Blockchain>>) {

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(2);
    ctx.lock().unwrap().add_swarm(2);
    ctx.lock().unwrap().add_swarm(2);

    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A1");
    ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A2");

    std::thread::sleep(std::time::Duration::from_millis(2000));

    ctx.lock().unwrap().check_messages();

}

/// Test that a dissolving swarm will push its data to other swarms
pub fn test_dissolving(bc: Arc<Mutex<Blockchain>>) {

    let mut ctx = TestContext::new(Arc::clone(&bc));

    ctx.add_swarm(1);
    ctx.add_swarm(1);
    ctx.add_swarm(1);

    // give SNs some time to initialize their servers
    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A");
    ctx.send_message("2b959eac778ee6bfac5e02c29800d489d319b65a9b8960a4cf4d3f40285b7735", "B");
    ctx.send_message("18b593e832ffda161c20a5daf842ab787ee7181a369ff7034fe80fb2774e0664", "C");
    ctx.send_message("17311f5ae7ce94b79698f12be6f3a2d66ec036fcf77506bf74877381630093af", "D");

    std::thread::sleep(std::time::Duration::from_millis(300));

    // Some messages will go to swarm 1, dissolve it
    // Now some of them will go to swarm 0, and the rest will go to swarm 2
    &bc.lock().unwrap().swarm_manager.dissolve_swarm(1);
    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A2");
    ctx.send_message("2b959eac778ee6bfac5e02c29800d489d319b65a9b8960a4cf4d3f40285b7735", "B2");
    ctx.send_message("18b593e832ffda161c20a5daf842ab787ee7181a369ff7034fe80fb2774e0664", "C2");
    ctx.send_message("17311f5ae7ce94b79698f12be6f3a2d66ec036fcf77506bf74877381630093af", "D2");

    std::thread::sleep(std::time::Duration::from_millis(2000));
    ctx.check_messages();

}

/// 4. Test a node going offline without updating the swarm list
pub fn test_snode_disconnecting(bc: Arc<Mutex<Blockchain>>) {

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(2);

    std::thread::sleep(std::time::Duration::from_millis(300));

    ctx.lock().unwrap().send_random_message();
    ctx.lock().unwrap().send_random_message();
    ctx.lock().unwrap().send_random_message();

    ctx.lock().unwrap().disconnect_snode();

    ctx.lock().unwrap().send_random_message();
    ctx.lock().unwrap().send_random_message();
    ctx.lock().unwrap().send_random_message();

    std::thread::sleep(std::time::Duration::from_millis(2000));

    ctx.lock().unwrap().check_messages();

}

pub fn test_blocks(bc : Arc<Mutex<Blockchain>>) {

    let mut rng = StdRng::seed_from_u64(0);

    // Generate 100 users
    let mut pks = vec![];

    for _ in 0..100 {
        let pk = PubKey::gen_random(&mut rng);
        pks.push(pk);
    }

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(3);

    // Every iteration in this loop corresponds to a block
    for i in 0..100 {

        // how much to wait until the next block
        let ms = rng.gen_range(500, 2000);
        std::thread::sleep(std::time::Duration::from_millis(ms));

        // deregister some
        if ctx.lock().unwrap().snode_count() > 10 {
            let n = rng.gen_range(0, 3);
            for _ in 0..n {
                ctx.lock().unwrap().drop_snode();
            }
        }

        // register some
        if ctx.lock().unwrap().snode_count() < 50 {
            let n = rng.gen_range(0, 3);
            for _ in 0..n {
                ctx.lock().unwrap().add_snode();
            }
        }

        println!("iteration: {}", i);
        println!("swarms: {}", ctx.lock().unwrap());

    }

    ctx.lock().unwrap().print_stats();


}

pub fn large_test(bc: Arc<Mutex<Blockchain>>) {

    let mut rng = StdRng::seed_from_u64(0);

    // start with a limited pool of public keys
    let pks = [
        "ef57493cd18b632c7d4351f1bc91c23f62f933479e9fd5a976a6f2912edf71a3",
        "f63ca4260ffae0512e4f9f84435d7d76e41609d1ccf61507d67f9337c36c11a8",
        "1efe23d3a5445abe6f7a74c0601f352699d82a33020defa8c6f72f9cd1d403d3",
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351",
        "648763a60ef63e49988a3e3934338745ff45d6b8410181eab58caf92ec97ae27",
    ];

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(3);

    let ctx_clone = ctx.clone();

    let node_thread = std::thread::spawn(move || {

        for i in 0..100 {
        // create a node every second
            std::thread::sleep(std::time::Duration::from_millis(1000));

            let num = rng.gen_range(5, 10);
            if ctx_clone.lock().unwrap().snode_count() > num {
                ctx_clone.lock().unwrap().drop_snode();
            }

            ctx_clone.lock().unwrap().add_snode();
        }
    });

    let ctx_clone = ctx.clone();

    let message_thread = std::thread::spawn(move || {

        let mut rng = StdRng::seed_from_u64(0);

        std::thread::sleep(std::time::Duration::from_millis(50));

        for _ in 0..2000 {
            // give SNs some time to initialize their servers
            std::thread::sleep(std::time::Duration::from_millis(50));

            let pk = pks.choose(&mut rng).unwrap();

            ctx_clone.lock().unwrap().send_random_message_to_pk(&pk);
        };

    });

    node_thread.join().unwrap();
    message_thread.join().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(2000));

    ctx.lock().unwrap().check_messages();


}

pub fn test_with_wierd_clients(bc: Arc<Mutex<Blockchain>>) {

    let sm = &mut bc.lock().unwrap().swarm_manager;

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    let bc = Arc::clone(&bc);

    sm.add_swarm(&[1]);

    std::thread::spawn(move || {
        // give SNs some time to initialize their servers
        std::thread::sleep(std::time::Duration::from_millis(200));


        // Construct an unreasonably large message:
        let mut large_msg = String::new();

        // NOTE: Our server fails on this (Error(9): body limit exceeded)
        for _ in 0..200000 {
            large_msg.push_str("012345657");
        }

        ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", &large_msg);

        std::thread::sleep(std::time::Duration::from_millis(2000));

        ctx.lock().unwrap().check_messages();

        std::thread::sleep(std::time::Duration::from_millis(2000));

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

fn test_small_random(bc: Arc<Mutex<Blockchain>>) {

    let ctx = TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_swarm(3);

    let ctx_clone = ctx.clone();

    let node_thread = std::thread::spawn(move || {

        std::thread::sleep(std::time::Duration::from_millis(1000));
        ctx_clone.lock().unwrap().add_snode();

        ctx_clone.lock().unwrap().send_random_message();
        std::thread::sleep(std::time::Duration::from_millis(100));
        ctx_clone.lock().unwrap().send_random_message();
        std::thread::sleep(std::time::Duration::from_millis(100));
        ctx_clone.lock().unwrap().send_random_message();
        std::thread::sleep(std::time::Duration::from_millis(100));

        ctx_clone.lock().unwrap().drop_snode();

        ctx_clone.lock().unwrap().add_snode();
        ctx_clone.lock().unwrap().add_snode();
        ctx_clone.lock().unwrap().add_snode();


    });

    node_thread.join().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(2000));

    ctx.lock().unwrap().check_messages();

}