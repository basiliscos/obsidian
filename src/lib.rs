extern crate bytebuffer;

use bytebuffer::ByteBuffer;
use std::cell::RefCell;
use std::rc::Rc;

// http://tailhook.github.io/netbuf/netbuf/struct.Buf.html

use std::vec::Vec;

pub trait EventStream<F> {
  fn push_write(&self, buf: &[u8]);
  fn set_read(&mut self, f: F)
    where F : Fn(&EventStream<F>);
  fn get_buf<'a>(&'a self) -> &'a RefCell<ByteBuffer>;
}


pub struct ByteEventSteam<F> {
  write_buff: Rc<RefCell<ByteBuffer>>,
  read_buff: Rc<RefCell<ByteBuffer>>,
  read_cb: Option<F>,
}

impl<F> ByteEventSteam<F> {
  fn new() -> Self
    where F : Fn(&EventStream<F>)
  {
    ByteEventSteam {
      write_buff: Rc::new(RefCell::new(ByteBuffer::new())),
      read_buff: Rc::new(RefCell::new(ByteBuffer::new())),
      read_cb: None,
    }
  }

  fn trigger_read_cb(&self)
    where F : Fn(&EventStream<F>)
  {
    if let Some(cb) = self.read_cb.as_ref() {
      cb(self);
    }
  }
}

impl<F> EventStream<F> for ByteEventSteam<F> {
  fn push_write(&self, buf: &[u8]) {
    let mut write_buff = self.write_buff.borrow_mut();
    write_buff.write_bytes(buf);
  }

  fn set_read(&mut self, f: F) {
    self.read_cb = Some(f);
  }

  fn get_buf<'a>(&'a self) -> &'a RefCell<ByteBuffer> {
    &self.read_buff
  }

}

pub struct DummyReactor<'a, F:'a, G:'a>
  where G: Fn(&ByteEventSteam<F>) -> bool,
{
  actions: Vec<(G, &'a ByteEventSteam<F>)>,
  streams: Vec<&'a ByteEventSteam<F>>,
}


impl<'a, F:'a, G:'a> DummyReactor<'a, F, G> where
  F: Fn(&EventStream<F>),
  G: Fn(&ByteEventSteam<F>) -> bool,
  {

  fn new() -> Self {
    DummyReactor {
      streams: Vec::new(),
      actions: Vec::new(),
    }
  }

  fn register(&mut self, stream: &'a ByteEventSteam<F>)  {
    self.streams.push(stream);
  }

  fn push_action(&mut self, stream: &'a ByteEventSteam<F>, action: G) {
    self.actions.push((action, stream));
  }

  fn play(&self) {
    for &(ref action, ref stream) in &self.actions {
      action(stream);
      stream.trigger_read_cb();
    }
  }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_echo() {
      let mut bes = ByteEventSteam::new();

      bes.set_read(|stream| {
        let mut read_buff = stream.get_buf().borrow_mut();
        let data = read_buff.to_bytes();
        let request = unsafe { String::from_utf8_unchecked(data) };
        assert_eq!(request, "ping?");
        read_buff.clear();

        stream.push_write(&String::from("pong!").into_bytes());
      });

      let mut dr = DummyReactor::new();

      dr.push_action(&bes, |stream| {
        let mut read_buff = stream.get_buf().borrow_mut();
        read_buff.write_bytes(&String::from("ping?").into_bytes());
        false
      });

      dr.register(&bes);

      dr.play();
      let response = unsafe { String::from_utf8_unchecked(bes.write_buff.borrow().to_bytes()) };
      assert_eq!(response, "pong!");

    }
}
