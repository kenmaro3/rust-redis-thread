extern crate redis;
use redis::Commands;
use std::thread;
use std::time::{Duration, Instant};

use std::sync::{
    mpsc::{self, Receiver, Sender, SyncSender},
    Mutex,
};

use once_cell::sync::Lazy;
static SHARE_STATE: Lazy<Mutex<Option<u64>>> = Lazy::new(|| Mutex::new(None));

//static SHARE_STATE_DATA: Lazy<(Mutex<mpsc::Sender<Vec<u8>>>, Mutex<mpsc::Receiver<Vec<u8>>>)> =
static SHARE_STATE_DATA: Lazy<(Mutex<mpsc::Sender<u8>>, Mutex<mpsc::Receiver<u8>>)> =
    Lazy::new(|| {
        //let (s, r): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel();
        let (s, r): (mpsc::Sender<u8>, mpsc::Receiver<u8>) = mpsc::channel();

        let tx = Mutex::new(s);
        let rx = Mutex::new(r);

        (tx, rx)

        //(s, r)
    });

fn do_something() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_connection()?;

    /* do something here */
    // set my_key 10
    let _: () = con.set("my_key", 10)?;
    // read my_key
    let result: i32 = con.get("my_key")?;
    println!("my_key: {}", result);

    Ok(())
}

// function for returning redis connection
fn get_connection() -> redis::RedisResult<redis::Connection> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    return client.get_connection();
}

fn write_mock(x: u8) {
    // sleep 2 seconds
    thread::sleep(Duration::from_secs(2));
    &SHARE_STATE_DATA.0.lock().unwrap().send(x).unwrap();
}

fn attempt1() {
    println!("share state waiting");
    // let state = SHARE_STATE.lock().unwrap();
    let data1: u8 = 20;
    write_mock(data1);

    println!("will crete handler thread");
    let handler = thread::spawn(move || {
        // this will block until the previous message has been received
        let message = SHARE_STATE_DATA.1.lock().unwrap();
        let data = message.recv().unwrap();
        println!("GOT data: {data}");
        return data;
    });
    // join handler thread
    println!("will join handler thread");

    let data = handler.join().unwrap();
    println!("got data from handler thread: {}", data);
}

fn redis_get_set() {
    // get connection
    let mut con = get_connection().unwrap();
    // set my_key as 20
    let _: () = con.set("my_key", 20).unwrap();
    // get my_key
    let result: i32 = con.get("my_key").unwrap();
    println!("my_key: {}", result);
}

fn attempt2() {
    let mut con = get_connection().unwrap();

    // pub sub of redis
    // in subthread, publish a message to key_message
    let handler = std::thread::spawn(move || {
        // get redis connection as con_thread
        let mut con_thread = get_connection().unwrap();

        let mut pubsub = con_thread.as_pubsub();
        pubsub.subscribe("data").unwrap();
        let msg = pubsub.get_message().unwrap();
        let payload: u8 = msg.get_payload().unwrap();
        //println!("channel '{}': {}", msg.get_channel_name(), payload);
        return payload;
    });
    // in main thread, sleep 2 seconds and write 30 to key: data
    // sleep 2 seconds
    println!("will sleep 2 seconds");
    thread::sleep(Duration::from_secs(2));
    println!("will write 30 to key: data");
    // write 30 to key_message
    let _: () = con.publish("data", 30).unwrap();

    let data = handler.join().unwrap();
    println!("got data from handler thread: {}", data);
}

fn attempt3() -> redis::RedisResult<()> {
    let mut con = get_connection().unwrap();

    //con.set("data", 0 as u8).unwrap();
    // let _: () = con.set("my_key", 10)?;
    let _: () = con.set("data", 10)?;
    // pub sub of redis
    // in subthread, publish a message to key_message
    let handler = std::thread::spawn(move || {
        // get redis connection as con_thread
        let mut con_thread = get_connection().unwrap();

        //let mut pubsub = con_thread.as_pubsub();
        loop {
            let result: u8 = con_thread.get("data").unwrap();
            println!("data: {}", result);
            // sleep 1 second
            //thread::sleep(Duration::from_secs(1));
            println!("sleeping...");
            return result;
        }
    });
    // in main thread, sleep 2 seconds and write 30 to key: data
    // sleep 2 seconds
    println!("will sleep 2 seconds");
    thread::sleep(Duration::from_secs(3));
    println!("will write 30 to key: data");
    // write 30 to key_message
    //con.set("data", 30).unwrap();
    let _: () = con.set("data", 30)?;

    let data = handler.join().unwrap();
    //println!("got data from handler thread: {}", data);
    Ok(())
}

fn main() {
    //do_something();

    // attempt1 is using mpsc (shared memory)
    // attmpet2 is using redis pubsub
    // what we want to achive is to let main thread write to some shared memory then the handler thread will read from it, then join the handler thread in to main thread
    attempt2();
}
