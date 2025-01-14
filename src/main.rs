#![allow(
    dead_code,
    unused_variables,
    unused_imports)]

use vulkanalia::VkResult;
use vulkanalia::vk;

mod context;
use context::Context;
use context::DeviceF32Array;

mod process_unit;
use process_unit::ScalarArgType;
use process_unit::ScalarArgVal;
use process_unit::Factory;
use process_unit::FactoryObject;
use process_unit::FactoryObjectBase;
// A struct for making `program` entities

//fn main(){
//    println!("FUCK RUST. ITS NOT A 'CRAB' ITS UTTTERLY CRAP");
//}

#[derive(Debug)]
struct MulBy2<'a>{
    pub alen: u32,
    pub base: FactoryObjectBase<'a>,
}

impl MulBy2<'_>{
    pub fn out(&self) -> &DeviceF32Array{
	self.base.get_output(0)
    }

}


// A static fxn that takes in the 'knob' struct and produces valuable information
#[derive(Copy, Clone)]
pub struct MulBy2InitArgs{
    pub arr_size: u32,
}

#[derive(Copy, Clone)]
pub struct  MulBy2CallArgs<'a>{
    pub x_in: &'a DeviceF32Array,
}

impl<'a> FactoryObject<'a> for MulBy2<'a>{
    const INPUT_ARRAY_COUNT: usize = 1;
    const INPUT_SCALAR_COUNT: usize = 1;
    const OUTPUT_ARRAY_COUNT: usize = 1;
    const INPUT_SCALAR_TYPES: &'static [ScalarArgType] = &[ScalarArgType::ArrayLen];

    type Knobs = MulBy2InitArgs;
 
    fn input_array_sizes(knobs: Self::Knobs) -> Vec<u32>{ vec![knobs.arr_size] }
    fn output_array_sizes(knobs: Self::Knobs) -> Vec<u32>{ vec![knobs.arr_size] }

    fn factory(ctx: &'a Context) -> VkResult<Factory<'a>> {
	use vulkanalia::bytecode::Bytecode;
	let code = Bytecode::new(include_bytes!("../shaders/add_arr.comp.spv")).unwrap();
	return Factory::new::<Self>(ctx, &code);
    }
    fn new(base_obj: FactoryObjectBase<'a>, knobs: Self::Knobs) -> VkResult<Self>{
	Ok(Self{
	    base: base_obj,
	    alen: knobs.arr_size,
	})
    }
    type Inputs = MulBy2CallArgs<'a>;
    fn exec_cmd(&mut self, cmd_buf: &vk::CommandBuffer, args: Self::Inputs) {

	use vulkanalia::prelude::v1_0::*;
	use vulkanalia::prelude::v1_1::*;
	use vulkanalia::prelude::v1_2::*;
	use vulkanalia::prelude::v1_3::*;

	//println!("This is just at the start of exec_cmd : {:#?}", self);
	
	self.base.write_input(0, args.x_in);
	self.base.write_scalar(0, ScalarArgVal::ArrayLen(args.x_in));
	//println!("This is just before executing setup_pre_cmd in exec_cmd : {:#?}", self);
	self.base.setup_pre_cmd(cmd_buf);
	//println!("This is just after executing setup_pre_cmd in exec_cmd : {:#?}", self);
        let ctx = self.base.ctx().unwrap(); // Should we signal error ??
	// calc optimal dispatch count
	const LOCAL_SIZE_X:u32 = 64;
	let group_x = (self.alen + LOCAL_SIZE_X - 1) / LOCAL_SIZE_X;
	unsafe{ctx.dev.cmd_dispatch(*cmd_buf, group_x, 1, 1)};
    }
    
    
}


fn main(){
    use vulkanalia::bytecode::Bytecode;
    use vulkanalia::prelude::v1_0::*;
    use vulkanalia::prelude::v1_1::*;
    use vulkanalia::prelude::v1_2::*;
    use vulkanalia::prelude::v1_3::*;
    let ctx = Context::new().unwrap();
    
    // View the device limits
    // let phy_props = unsafe{cxt.inst.get_physical_device_properties(cxt.phy_dev)};
    // println!("The physical device properties are {:#?}", phy_props);

    //let mul_2er = DevOperation::new(&cxt, 1, 0, &Bytecode::new(include_bytes!("../shaders/add_arr.comp.spv")).unwrap()).unwrap();
    //let mul_2er = DevOperation::new(&cxt, 1, &vec![PushConstType::ArrayLen, PushConstType::ArrayLen], &Bytecode::new(include_bytes!("../shaders/add_arr.comp.spv")).unwrap()).unwrap();


    let fac_2x = MulBy2::factory(&ctx).unwrap();
    
    // allocate input buffer
    let arr_len:u32 = 32;
    let buff_in = ctx.new_array(arr_len as usize, false).unwrap();
    // Write the data
    {
        // TODO:: Find if the length is correct
        let input_data = [
            1.0,   2.0,   3.0,   4.0,   5.0,   6.0,   7.0,   8.0,
            9.0,   10.0,   11.0,   12.0,   13.0,   14.0,   15.0,   16.0,  
            -1.0, -2.0,  -3.0,  -4.0,  -5.0,  -6.0,  -7.0,  -8.0,
            -9.0,  -10.0,  -11.0,  -12.0,  -13.0,  -14.0,  -15.0,  -16.0,
        ] as [f32;32];
        ctx.write_array(&buff_in, &input_data);
        println!("Mapped memory of buff1");
    }


    let mut obj_2x = fac_2x.produce::<MulBy2>(MulBy2InitArgs{
	arr_size: arr_len
    }).unwrap();

    // Command buffer recording
    unsafe{ctx.dev.begin_command_buffer(ctx.cmd_buff, &vk::CommandBufferBeginInfo::builder()).unwrap()};
    obj_2x.exec_cmd(&ctx.cmd_buff, MulBy2CallArgs{
	x_in : &buff_in,
    });
    unsafe{ctx.dev.end_command_buffer(ctx.cmd_buff).unwrap()};
    println!("Recorded the command buffers with {} elements in the array", arr_len);


    // Print the data too
    {
        //let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        let arr = ctx.read_array(&buff_in);
        println!("The array was \n{:?}", arr);
    }   

    for _ in 0..5{
	// Play command buffer
	unsafe{ctx.dev.queue_submit(ctx.comp_queue,
				    &[vk::SubmitInfo::builder().command_buffers(&[ctx.cmd_buff])],
				    vk::Fence::null()).unwrap()};
	println!("Played the command buffer");
	// Wait
	unsafe{ctx.dev.device_wait_idle().unwrap()};
	
	// Print data
	{
            //let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
            let arr = ctx.read_array(obj_2x.out());
            println!("The answer was \n{:?}", arr);
	}   
	// Print the data too
	{
            let arr = ctx.read_array(&buff_in);
            println!("The original array is \n{:?}", arr);
	}
	// Write data of buff2 into buff1
	unsafe{ctx.dev.begin_command_buffer(ctx.copy_cmd_buff, &vk::CommandBufferBeginInfo::builder()).unwrap()};
	unsafe{ctx.dev.cmd_copy_buffer
	       (ctx.copy_cmd_buff, obj_2x.out().buffer, buff_in.buffer,
		&[vk::BufferCopy{src_offset : 0,
			     dst_offset : 0,
			     size : buff_in.size as u64}]
	)};
	unsafe{ctx.dev.end_command_buffer(ctx.copy_cmd_buff).unwrap()};	
	unsafe{ctx.dev.queue_submit(ctx.comp_queue,
				    &[vk::SubmitInfo::builder().command_buffers(&[ctx.copy_cmd_buff])],
				    vk::Fence::null()).unwrap()};
	println!("Copied buff2->buff1 in the command buffer");
	println!("\n-------------------------------------------------------\n");
	// Wait
	unsafe{ctx.dev.device_wait_idle().unwrap()};
	
    }
    unsafe{ctx.dev.device_wait_idle().unwrap()};
    // Print everything once

    //println!("The obj_2x is \n{:#?}", obj_2x);
    //println!("The buff_in is \n{:#?}", buff_in);
    ctx.drop_array(&buff_in);

}


