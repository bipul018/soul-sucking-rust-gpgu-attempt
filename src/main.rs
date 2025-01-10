#![allow(
    dead_code,
    unused_variables,
    unused_imports)]

use vulkanalia::VkResult;
use vulkanalia::vk;

mod context;
use context::Context;

mod process_unit;
use process_unit::DevOperation;
// A struct for making `program` entities

fn main(){
    use vulkanalia::bytecode::Bytecode;
    use vulkanalia::prelude::v1_0::*;
    use vulkanalia::prelude::v1_1::*;
    use vulkanalia::prelude::v1_2::*;
    use vulkanalia::prelude::v1_3::*;
    let cxt = Context::new().unwrap();

    let mul_2er = DevOperation::new(&cxt, 1, 0, &Bytecode::new(include_bytes!("../shaders/add_arr.comp.spv")).unwrap()).unwrap();
    
    // allocate buffers
    let arr_len:u32 = 32;
    let buff1 = cxt.new_array(arr_len as usize, false).unwrap();
    let buff2 = cxt.new_array(arr_len as usize, false).unwrap();
    println!("The memory index chosen for buffers is {}", cxt.vis_buff_mem_type);
    
    println!("Memories for buffer were allocated!");

    // Command buffer recording
    unsafe{cxt.dev.begin_command_buffer(cxt.cmd_buff, &vk::CommandBufferBeginInfo::builder()).unwrap()};
    mul_2er.record_cmd(&[buff1], buff2, &([] as [f32;0]));
    unsafe{cxt.dev.end_command_buffer(cxt.cmd_buff).unwrap()};
    println!("Recorded the command buffers with {} elements in the array", arr_len);

    // Write the data
    {
        // TODO:: Find if the length is correct
        let input_data = [
            1.0,   2.0,   3.0,   4.0,   5.0,   6.0,   7.0,   8.0,
            9.0,   10.0,   11.0,   12.0,   13.0,   14.0,   15.0,   16.0,  
            -1.0, -2.0,  -3.0,  -4.0,  -5.0,  -6.0,  -7.0,  -8.0,
            -9.0,  -10.0,  -11.0,  -12.0,  -13.0,  -14.0,  -15.0,  -16.0,
        ] as [f32;32];
        cxt.write_array(&buff1, &input_data);
        println!("Mapped memory of buff1");
    }
    // Print the data too
    {
        //let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        let arr = cxt.read_array(&buff1);
        println!("The array was \n{:?}", arr);
    }   
    
    // Play command buffer
    unsafe{cxt.dev.queue_submit(cxt.comp_queue,
        &[vk::SubmitInfo::builder().command_buffers(&[cxt.cmd_buff])],
        vk::Fence::null()).unwrap()};
    println!("Played the command buffer");
    // Wait
    unsafe{cxt.dev.device_wait_idle().unwrap()};
    
    // Print data
    {
        //let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        let arr = cxt.read_array(&buff2);
        println!("The answer was \n{:?}", arr);
    }   
    // Print the data too
    {
        let arr = cxt.read_array(&buff1);
        println!("The original array is \n{:?}", arr);
    }   
    

    unsafe{cxt.dev.device_wait_idle().unwrap()};
    cxt.drop_array(&buff2);
    cxt.drop_array(&buff1);
}
