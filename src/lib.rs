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
  fn get_buf<'a>(&'a self) -> &'a RefCell<MutByteBuf>;
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

  fn get_buf<'a>(&'a self) -> &'a RefCell<MutByteBuf> {
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
    /*
    for bes in &self.streams {
      bes.trigger_read_cb();
    }
    */
  }
}


#[cfg(test)]
mod tests {
    use super::*;
    use bytes::str::Bytes;
    use bytes::str::ToBytes;
    use bytes::buf::{ByteBuf, MutByteBuf};
    use bytes::ByteStr;
    use bytes::buf::Source;

    use std::ops::Deref;


    #[test]
    fn simple_echo() {
      let mut bes = ByteEventSteam::new(1024);

      let mut dr = DummyReactor::new();

      bes.push_write(Bytes::from_slice(&String::from("ping?").into_bytes()));
      bes.set_read(|stream| {
        let read_buff = stream.get_buf().borrow_mut();
        let data = read_buff.bytes();

        let mut vector_data =  Vec::new();
        vector_data.extend_from_slice(data);
        let request = unsafe { String::from_utf8_unchecked(vector_data) };
        assert_eq!(request, "ping?");
      });

      dr.push_action(&bes, |stream| {
        let mut read_buff = stream.get_buf().borrow_mut();
        let new_bytes = Bytes::from_slice(&String::from("ping?").into_bytes());
        let old_bytes = Bytes::from_slice(read_buff.bytes());
        let concat_bytes = old_bytes.concat(&new_bytes);
        let mut new_read_buff = ByteBuf::mut_with_capacity(concat_bytes.len());

        let result_buf = concat_bytes.buf();
        let result_bytes = result_buf.deref().bytes();
        assert_eq!(result_bytes, [112, 105, 110, 103, 63]);
        let result_read_buf = ByteBuf::from_slice(result_bytes);
        *read_buff = result_read_buf.resume();

        false
      });

      dr.register(&bes);


      dr.play();
    }
}
