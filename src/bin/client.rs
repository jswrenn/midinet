#[macro_use] extern crate structopt;
extern crate alsa;


use alsa::seq;
use alsa::poll::poll;
use alsa::PollDescriptors;
use std::error::Error;
use std::ffi::CString;
use std::fmt::Display;
use structopt::StructOpt;
use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};


type Port = i32;


#[derive(StructOpt)]
struct Options {
  /// addresses of clients to connect to
  #[structopt(name = "HOSTS")]
  hosts: Vec<String>,
}


fn input<A: ToSocketAddrs + Display + Clone>(seq: &alsa::Seq, host: A)
    -> Result<(Port, Vec<SocketAddr>), Box<Error>>
{
  let port =
    seq.create_simple_port(
      &CString::new(format!("{}", host)).unwrap(),
      seq::WRITE | seq::SUBS_WRITE,
      seq::MIDI_GENERIC | seq::APPLICATION)?;
  Ok((port, host.to_socket_addrs()?.collect()))
}


fn run(options : &Options) -> Result<!, Box<Error>> {
  let sequencer_name =
    CString::new(format!("{} client", env!("CARGO_PKG_NAME"))).unwrap();
  let sequencer = alsa::Seq::open(None, None, true)?;
  sequencer.set_client_name(&sequencer_name)?;

  let socket = UdpSocket::bind("0.0.0.0:0")?;

  let mut sinks = Vec::with_capacity(options.hosts.len());

  for (i, host) in options.hosts.iter().enumerate() {
    let (port, addrs) = input(&sequencer, host)?;
    assert_eq!(i as i32, port);
    sinks.push(addrs);
  }

  let mut input = sequencer.input();
  let mut buffer : [u8; 12] = [0; 12];
  let coder = seq::MidiEvent::new(0)?;
  coder.enable_running_status(false);

  let mut fds = (&sequencer, Some(alsa::Direction::input())).get()?;

  loop {
    if input.event_input_pending(true)? == 0 {
      poll(fds.as_mut_slice(), -1)?;
      continue;
    } else {
      let mut event = input.event_input()?;
      let destination = event.get_dest();
      if let Ok(bytes) = coder.decode(&mut buffer[..], &mut event) {
        socket.send_to(&buffer[0..bytes],
          sinks[destination.port as usize].as_slice())?;
      }
    }
  }
}


fn main() {
  let options = Options::from_args();
  // run and, if necessary, print error message to stderr
  if let Err(error) = run(&options) {
    eprintln!("Error: {}", error);
    std::process::exit(1);
  }
}