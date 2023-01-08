use std::io::{Read};
use std::mem::MaybeUninit;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::{env, thread};
use std::time::Duration;
use cpal::traits::HostTrait;
use cpal::traits::{DeviceTrait};
use cpal::Data;
use cpal::traits::{StreamTrait};
use ringbuf::{Consumer, HeapRb, Producer, SharedRb};

type MyProd<T> = Producer<T, Arc<SharedRb<T, Vec<MaybeUninit<T>>>>>;
type MyCons<T> = Consumer<T, Arc<SharedRb<T, Vec<MaybeUninit<T>>>>>;

fn main() {
    let args: Vec<String> = env::args().collect();

    let adresse_of_audio_server = &args[1];

    //"127.0.0.1:7878"
    println!("addres of audio server is {}", adresse_of_audio_server);

    // let (tx ,rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = std::sync::mpsc::channel();
    let ringbuffer_wait_time = 200;
    //frames * ringbuffer wiaitng time
    let (mut prod, mut consumer): (MyProd<f32>, MyCons<f32>)  = HeapRb::<f32>::new( 960*10000).split();
    //frames * ringbuffer wiaitng time
    //at a herty of 48000 and 960 buffer, 50 buffer equal 1 second (?) i think
    //lies: actually we still have 2 channels in input so its 100 buffer a second i think (seems to be correct)
    //and maybe 2 frames? idk at this pooint im guesssing so. 400 seems to be equal to 2 seconds
    let empty_sound_wait: Vec<f32> = vec![0_f32;960*500];
    prod.push_iter(&mut empty_sound_wait.into_iter());



    // if let Ok(server) = TcpListener::bind("127.0.0.1:7878") {
    if let Ok(stream) = TcpStream::connect(adresse_of_audio_server) {
        thread::spawn(move || {
            handle_connections_rb_as_listener(stream, &mut prod);
            //old
            // handle_connections_rb(server, &mut prod);
        });
    } else {
        println!("server could not be started");
    }

    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device available");

    let mut supported_configs_range = device.supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range.next()
        .expect("no supported config?!")
        .with_max_sample_rate();


    let stream = device.build_output_stream(
        &supported_config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // readFromRingBuffer(data, &buffer);
             for (left, right) in data.iter_mut().zip(consumer.pop_iter()) {
                 // println!("{right}");
                 *left = right;
             }
            // if let Ok(audio) = rx.recv() {
            //     for (left, right) in data.iter_mut().zip(audio) {
            //     }
            // }
// react to stream events and read or write stream data here.
        },
        move |err| {
// react to errors here.
        },
    ).expect("should haeve build stream but didnt lol");

    stream.play().unwrap();
    thread::sleep(Duration::from_secs(200));

}
fn handle_connections_rb_as_listener(stream: TcpStream, tx: &mut MyProd<f32>) {
    read_bytes_from_stream_rb(stream, tx);
}
fn handle_connections_rb(server: TcpListener, tx: &mut MyProd<f32>) {
    for connection in server.incoming() {
        if let Ok(stream) = connection {
            println!("connection succesfull");
            read_bytes_from_stream_rb(stream, tx);
        } else {
            println!("connection failed");
        }
    }
}

fn handle_connections(server: TcpListener, tx: &Sender<Vec<f32>>) {
    for connection in server.incoming() {
        if let Ok(stream) = connection {
            println!("connection succesfull");
            read_bytes_from_stream(stream, tx);
        } else {
            println!("connection failed");
        }
    }
}

fn read_bytes_from_stream_rb(mut stream: TcpStream, tx: &mut MyProd<f32>) {
    let mut buf: [u8; 960*4] = [0_u8; 960*4];
    loop {
        if let Ok(_bytes_read) = stream.read(&mut buf) {
            // if bytes_read == 0 {
            //     println!("bytes read was 0");
            //     break;
            // }
            // println!("iteration");
            handle_bytes_rb( buf, tx);
        } else {
            println!("stream couldnt read anymore");
            break;
        }
    }
}
fn read_bytes_from_stream(mut stream: TcpStream, tx: &Sender<Vec<f32>>) {
    let mut buf: [u8; 960*4] = [0_u8; 960*4];
    loop {
        if let Ok(bytes_read) = stream.read(&mut buf) {
            if bytes_read == 0 {
                println!("bytes read was 0");
                break;
            }
            // println!("iteration");
            handle_bytes(&mut buf, tx);
        } else {
            println!("stream couldnt read anymore");
            break;
        }
    }
}

fn handle_bytes_rb(raw_bytes: [u8; 960*4], tx: &mut MyProd<f32>) {
    let mut sound_floats = [0_f32; 960];
    for i in 0..960 {
       let float_in_chunks = &raw_bytes[i*4..i*4+4];
        let soundfloat = f32::from_be_bytes(<[u8; 4]>::try_from(float_in_chunks).unwrap());
        sound_floats[i] = soundfloat;
    }
    tx.push_iter(&mut sound_floats.into_iter());
    // let f32_in_bytechunks = raw_bytes.chunks(4);
    // let converted_to_f32: Vec<f32> = f32_in_bytechunks.map(|e | f32::from_be_bytes(<[u8; 4]>::try_from(e).unwrap())).collect();
    // tx.push_iter(&mut converted_to_f32.into_iter());
}
fn handle_bytes(buf: &mut [u8; 960*4], tx: &Sender<Vec<f32>>) {
    let f32_in_bytes = buf.chunks(4);
    let erg: Vec<f32> = f32_in_bytes.map(|e | f32::from_be_bytes(<[u8; 4]>::try_from(e).unwrap())).collect();
    // println!("{:?}", erg);
     tx.send(erg);
    // buf.iter().for_each(|e| {
    //     println!("bytes read: {e} ");
    // });
}
