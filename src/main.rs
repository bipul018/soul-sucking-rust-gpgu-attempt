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
struct Scalex<'a>{
    pub alen: u32,
    pub base: FactoryObjectBase<'a>,
}

impl Scalex<'_>{
    pub fn out(&self) -> &DeviceF32Array{
	self.base.get_output(0)
    }

}


// A static fxn that takes in the 'knob' struct and produces valuable information
#[derive(Copy, Clone)]
pub struct ScalexInitArgs{
    pub arr_size: u32,
}

#[derive(Copy, Clone)]
pub struct  ScalexCallArgs<'a>{
    pub x_in: &'a DeviceF32Array,
    pub factor: f32,
}

impl<'a> FactoryObject<'a> for Scalex<'a>{
    const INPUT_ARRAY_COUNT: usize = 1;
    const INPUT_SCALAR_COUNT: usize = Self::INPUT_SCALAR_TYPES.len();
    const OUTPUT_ARRAY_COUNT: usize = 1;
    const INPUT_SCALAR_TYPES: &'static [ScalarArgType] = &[ScalarArgType::ArrayLen, ScalarArgType::F32];

    type Knobs = ScalexInitArgs;
 
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
    type Inputs = ScalexCallArgs<'a>;
    fn exec_cmd(&mut self, cmd_buf: &vk::CommandBuffer, args: Self::Inputs) {

	use vulkanalia::prelude::v1_0::*;
	use vulkanalia::prelude::v1_1::*;
	use vulkanalia::prelude::v1_2::*;
	use vulkanalia::prelude::v1_3::*;

	//println!("This is just at the start of exec_cmd : {:#?}", self);
	
	self.base.write_input(0, args.x_in);
	self.base.write_scalar(0, ScalarArgVal::ArrayLen(args.x_in));
	self.base.write_scalar(1, ScalarArgVal::F32(args.factor));
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


    let scale_factory = Scalex::factory(&ctx).unwrap();
    
    // allocate input buffer
    let arr_len:u32 = 32;
    // var x
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


    // scalar1 
    let mut do2x = scale_factory.produce::<Scalex>(ScalexInitArgs{
	arr_size: arr_len
    }).unwrap();
    // scalar2
    let mut do3x = scale_factory.produce::<Scalex>(ScalexInitArgs{
	arr_size: arr_len
    }).unwrap();    

    // Command buffer recording
    unsafe{ctx.dev.begin_command_buffer(ctx.cmd_buff, &vk::CommandBufferBeginInfo::builder()).unwrap()};
    
    // First y = 2x
    do2x.exec_cmd(&ctx.cmd_buff, ScalexCallArgs{
	x_in : &buff_in,
	factor: 2.0,
    });
    // Then z = 3y
    do3x.exec_cmd(&ctx.cmd_buff, ScalexCallArgs{
	x_in : &do2x.out(),
	factor: 3.0,
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
	
	// Print data x,y,z
	{
            println!("x=\n{:?}", ctx.read_array(&buff_in));
	    println!("y=\n{:?}", ctx.read_array(do2x.out()));
	    println!("z=\n{:?}", ctx.read_array(do3x.out()));
	}
	
	// x = z
	unsafe{ctx.dev.begin_command_buffer(ctx.copy_cmd_buff, &vk::CommandBufferBeginInfo::builder()).unwrap()};
	unsafe{ctx.dev.cmd_copy_buffer
	       (ctx.copy_cmd_buff, do3x.out().buffer, buff_in.buffer,
		&[vk::BufferCopy{src_offset : 0,
			     dst_offset : 0,
			     size : buff_in.size as u64}]
	)};
	unsafe{ctx.dev.end_command_buffer(ctx.copy_cmd_buff).unwrap()};	
	unsafe{ctx.dev.queue_submit(ctx.comp_queue,
				    &[vk::SubmitInfo::builder().command_buffers(&[ctx.copy_cmd_buff])],
				    vk::Fence::null()).unwrap()};
	println!("\n-------------------------------------------------------\n");
	// Wait
	unsafe{ctx.dev.device_wait_idle().unwrap()};
	
    }
    unsafe{ctx.dev.device_wait_idle().unwrap()};
    // Print everything once

    ctx.drop_array(&buff_in);

}


