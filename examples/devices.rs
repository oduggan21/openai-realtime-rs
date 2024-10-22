use openai_realtime_utils as utils;

fn main() {
  let inputs = utils::device::get_available_inputs();
  println!("Available inputs: {}", inputs);

  let outputs = utils::device::get_available_outputs();
  println!("Available outputs: {}", outputs);
}