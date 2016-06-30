extern crate bytebuffer;

use bytebuffer::ByteBuffer;
use std::cell::RefCell;
use std::rc::Rc;

// http://tailhook.github.io/netbuf/netbuf/struct.Buf.html

use std::vec::Vec;

pub trait EventStream {
  fn push_write(&self, buf: &[u8]);
  fn set_read(&self, f: Box<Fn(&EventStream, Rc<RefCell<ByteBuffer>>)>);
}

pub struct ByteEventSteam {
  write_buff: Rc<RefCell<ByteBuffer>>,
  read_buff: Rc<RefCell<ByteBuffer>>,
  read_cb: RefCell<Option<Box<Fn(&EventStream, Rc<RefCell<ByteBuffer>>)>>>,
}

impl EventStream for ByteEventSteam {
  fn push_write(&self, buf: &[u8]) {
    let mut write_buff = self.write_buff.borrow_mut();
    write_buff.write_bytes(buf);
  }

  fn set_read(&self, f: Box<Fn(&EventStream, Rc<RefCell<ByteBuffer>>)>) {
    let mut cb = self.read_cb.borrow_mut();
    *cb = Some(f);
  }
}

impl ByteEventSteam {
  fn new() -> Self {
    ByteEventSteam {
      write_buff: Rc::new(RefCell::new(ByteBuffer::new())),
      read_buff: Rc::new(RefCell::new(ByteBuffer::new())),
      read_cb: RefCell::new(None),
    }
  }

  fn trigger_read_cb(&self) {
    let read_cb = self.read_cb.borrow();
    if let Some(cb) = read_cb.as_ref() {
      cb(self, self.read_buff.clone());
    }
  }
}

pub struct DummyReactor<'a>
{
  actions: Vec<(Box<Fn(&ByteEventSteam)>, Rc<&'a ByteEventSteam>)>,
}

impl<'a> DummyReactor<'a> {

  fn new() -> Self {
    DummyReactor {
      actions: Vec::new(),
    }
  }

  fn push_action(&mut self, stream: Rc<&'a ByteEventSteam>, action: Box<Fn(&ByteEventSteam)>) {
    self.actions.push((action, stream));
  }

  fn play(&self) {
      for &(ref action, ref stream) in &self.actions {
      let bes:&ByteEventSteam = stream.as_ref();
      action(bes);
      stream.trigger_read_cb();
    }
  }
}


pub struct PingProtocol<'a> {
  stream: &'a EventStream
}

impl<'a> PingProtocol<'a>
{
  fn new(mut stream: &'a EventStream) -> Self {


    stream.set_read(Box::new(|stream, rx_rc| {
      println!("!");

      let mut rx_buff = rx_rc.borrow_mut();

      let data = rx_buff.to_bytes();
      let request = unsafe { String::from_utf8_unchecked(data) };
      rx_buff.clear();

      stream.push_write(&String::from("pong!").into_bytes());
    }));

    PingProtocol { stream: stream }
  }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn simple_echo() {


      let bes = ByteEventSteam::new();
      let bes_rc = Rc::new(&bes);
      let ping = PingProtocol::new(&bes);

      let mut dr = DummyReactor::new();

      dr.push_action(bes_rc.clone(), Box::new(|stream| {
        let mut read_buff = stream.read_buff.borrow_mut();
        read_buff.write_bytes(&String::from("ping?").into_bytes());
      }));

      dr.push_action(bes_rc.clone(), Box::new(|stream| {
        let mut read_buff = stream.read_buff.borrow_mut();
        read_buff.write_bytes(&String::from("ping?").into_bytes());
      }));


      dr.play();

      let response = unsafe { String::from_utf8_unchecked(bes.write_buff.borrow().to_bytes()) };
      assert_eq!(response, "pong!pong!");
    }
}
