#[macro_use] extern crate structopt;
extern crate alsa;


use alsa::seq;
use std::error::Error;
use std::ffi::CString;
use structopt::StructOpt;
use std::net::UdpSocket;


#[derive(StructOpt)]
struct Options {
  /// address to bind to
  #[structopt(name = "ADDR")]
  addr: String,
}


fn run(options : Options) -> Result<!, Box<Error>> {
  let sequencer_name =
    CString::new(format!("{} server", env!("CARGO_PKG_NAME"))).unwrap();
  let sequencer = alsa::Seq::open(None, None, true)?;
  sequencer.set_client_name(&sequencer_name)?;

  let output_port =
    sequencer.create_simple_port(
      &CString::new(options.addr.clone()).unwrap(),
      seq::READ | seq::SUBS_READ,
      seq::MIDI_GENERIC | seq::APPLICATION)?;

  let socket = UdpSocket::bind(options.addr)?;

  let mut buffer : [u8; 12] = [0; 12];
  let mut coder = seq::MidiEvent::new(321)?;
  coder.enable_running_status(false);

  loop {
    if let Ok((bytes, _)) = socket.recv_from(&mut buffer[..]) {
      if let (_, Some(mut event)) = coder.encode(&mut buffer[0..bytes])? {
        event.set_source(output_port);
        event.set_subs();
        event.set_direct();
        sequencer.event_output(&mut event)?;
        sequencer.drain_output()?;
      }
    }
  }
}


fn main() {
  let options = Options::from_args();
  // run and, if necessary, print error message to stderr
  if let Err(error) = run(options) {
    eprintln!("Error: {}", error);
    std::process::exit(1);
  }
}