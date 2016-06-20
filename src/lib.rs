extern crate bytes;

use bytes::buf::{ByteBuf, MutByteBuf};
use bytes::buf::MutBuf;
use bytes::str::Bytes;
use bytes::ByteStr;
use std::cell::RefCell;
use std::rc::Rc;

use std::vec::Vec;

pub trait EventStream<F> {
  fn push_write(&mut self, buf: Bytes);
  fn set_read(&mut self, f: F)
    where F : Fn(&EventStream<F>);
  fn get_buf<'a>(&'a self) -> &'a Rc<RefCell<MutByteBuf>>;
}

pub struct ByteEventSteam<F> {
  write_buff: Bytes,
  read_buff: Rc<RefCell<MutByteBuf>>,
  read_cb: Option<F>,
}

impl<F> ByteEventSteam<F> {
  fn new(capacity: usize) -> Self
    where F : Fn(&EventStream<F>)
  {
    ByteEventSteam {
      write_buff: Bytes::empty(),
      read_buff: Rc::new(RefCell::new(ByteBuf::mut_with_capacity(capacity))),
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
  fn push_write(&mut self, buf: Bytes) {
    self.write_buff = self.write_buff.concat(&buf);
  }

  fn set_read(&mut self, f: F) {
    self.read_cb = Some(f);
  }

  fn get_buf<'a>(&'a self) -> &'a Rc<RefCell<MutByteBuf>> {
    &self.read_buff
  }

}

pub struct DummyReactor<'a, F> {
  actions: Vec<&'a Fn() -> bool>,
  streams: Vec<ByteEventSteam<F>>,
}

impl<'a, F> DummyReactor<'a, F> where
  F : Fn(&EventStream<F>),
  {

  fn new() -> Self {
    DummyReactor {
      streams: Vec::new(),
      actions: Vec::new(),
    }
  }

  fn register(&mut self, stream: ByteEventSteam<F>)  {
    self.streams.push(stream);
  }

  fn play(&self) {
    //where F : Fn(&EventStream<F>) {
    for bes in &self.streams {
      bes.trigger_read_cb();
    }
  }

}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::str::Bytes;

    #[test]
    fn simple_echo() {
      let mut dr = DummyReactor::new();
      let mut bes = ByteEventSteam::new(1024);

      bes.push_write(Bytes::from_slice(&String::from("ping?").into_bytes()));
      bes.set_read(|stream| {
        let read_buff = stream.get_buf().borrow_mut();
        let data = read_buff.bytes();
        let mut vector_data =  Vec::new();
        vector_data.extend_from_slice(data);
        let request = unsafe { String::from_utf8_unchecked(vector_data) };
        assert_eq!(request, "ping?");
        //read_buff.clear();
      });

      dr.register(bes);
      dr.play();
    }
}
