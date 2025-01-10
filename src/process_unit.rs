use vulkanalia::VkResult;
use vulkanalia::vk;

//mod context;
use crate::context::Context;
use crate::context::DeviceF32Array;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::prelude::v1_1::*;
use vulkanalia::prelude::v1_2::*;
use vulkanalia::prelude::v1_3::*;

// TODO:: Will have to make the struct 'reusable' so will have to take care of the pipeline barriers, descriptros maybe later

/*
Model for the shader program for now:
1 >  All the shaders will have a single descriptor set
2 >  All the shaders will have first N (>= 1) descriptor as the read only array arguments then a single descriptor for a write only return array
3 >  All the shaders will also have N push constants denoting size of array by u32, and then 1 push constant denoting size of return array also by u32
4 >  All the shaderw will have the required scalar arguments if needed as f32 just after the u32 sizes 
5 >  This means there is a minimum space for 31 arguments, since the push constant can hold 32 * 4 bytes at minimum (given that there is enough per stage descriptor binding)

TODO:: Need to model also scalar returning shaders ??
*/

// A struct for making `program` entities
#[derive(Default)]
pub struct DevOperation<'a>{
    pub desc_layout: vk::DescriptorSetLayout,
    pub pipe_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub desc_pool: vk::DescriptorPool,
    pub desc_set: vk::DescriptorSet,
    pub array_arg_count: u32,
    pub scalar_arg_count: u32,
    

    //inputs: Vec<DeviceF32Array>,
    //output: Vec<DeviceF32Array>,

    //cleanup: Vec<fn(&mut DevOperation, &Context)>,
    cleanup: Vec<fn(&mut DevOperation<'a>, &'a Context)>,
    ctx: Option<&'a Context>,
}

impl Drop for DevOperation<'_>{
    fn drop(&mut self){
	self.clean();
    }
}
use vulkanalia::bytecode::Bytecode;
impl<'a> DevOperation<'a>{
    //pub fn clean(&mut self, cxt: &Context){
    pub fn clean(&mut self){
	match self.ctx{
	    None => {},
	    Some(ctx) => {
		let cleanup_functions: Vec<fn(&mut DevOperation<'a>, &'a Context)> =
		    self.cleanup.clone().into_iter().rev().collect();
		for cleanup_fn in cleanup_functions {
		    cleanup_fn(self, ctx);
		}
	    }
	}
    }
    
    pub fn new(ctx: &'a Context, arr_args: u32, scalar_args: u32, shader_code: &Bytecode) -> VkResult<DevOperation<'a>>{

        let mut this = DevOperation::default();
	this.ctx = Some(ctx);
	this.scalar_arg_count = scalar_args;

        macro_rules! defer{
            ($iden1: ident, $iden2: ident, $expr: expr) => {
                this.cleanup.push(|$iden1, $iden2|{ unsafe{ $expr; } });
            }
        }

        let desc_bindings:Vec<vk::DescriptorSetLayoutBindingBuilder> = (0..(arr_args+1)).map(|i|{
            vk::DescriptorSetLayoutBinding::builder().
                binding(i).
                descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
                descriptor_count(1).
                stage_flags(vk::ShaderStageFlags::COMPUTE)
        }).collect();
        
        this.desc_layout = unsafe{ctx.dev.create_descriptor_set_layout
        (&vk::DescriptorSetLayoutCreateInfo::builder().
            // TODO:: Find if different types of descriptors need to be set for readonly/readwrite glsl buffers
            bindings(&desc_bindings),
            None)?};
        println!("Descriptor set layout was created!");
        defer!(s,cxt, cxt.dev.destroy_descriptor_set_layout(s.desc_layout, None));
        
        this.pipe_layout = unsafe{ctx.dev.
            create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder().
                    set_layouts(&[this.desc_layout]).
                    push_constant_ranges(&[vk::PushConstantRange::builder().
                        stage_flags(vk::ShaderStageFlags::COMPUTE).
                        offset(0).
                        size(4 * (scalar_args + 1 + arr_args))]),
                None)?};
        println!("The pipeline layout was created!");
        defer!(s, ctx, ctx.dev.destroy_pipeline_layout(s.pipe_layout, None));

        let comp_shader_mod = unsafe{ctx.dev.
            create_shader_module(
                &vk::ShaderModuleCreateInfo::builder().
                    code_size(shader_code.code_size()).
                    code(shader_code.code()),
                None)}?;
        // Create the pipeline , then destroy the shader module
	let compute_info = [
            vk::ComputePipelineCreateInfo::builder().
		layout(this.pipe_layout).
		stage(vk::PipelineShaderStageCreateInfo::builder().
                      stage(vk::ShaderStageFlags::COMPUTE).
                      module(comp_shader_mod).
                      name(b"main\0"))
	];
	this.pipeline = {
	    let compute_pipes = unsafe{ctx.dev.
				       create_compute_pipelines(
					   vk::PipelineCache::null(),
					   &compute_info,
					   None)};
	    unsafe{ctx.dev.destroy_shader_module(comp_shader_mod, None)};
	    compute_pipes
	}?.0[0];
	println!("Created the compute pipeline and then destroyed the shader module!");
	defer!(this, ctx, ctx.dev.destroy_pipeline(this.pipeline, None));
	
        // Create the descriptor pool and allocate/bind descriptors    
	// TODO:: Later might need to let this happen at a `late stage` for allowing sharing of this entity
	this.desc_pool = unsafe{ctx.dev.
			       create_descriptor_pool(
				   &vk::DescriptorPoolCreateInfo::builder().
				       max_sets(1).
				       pool_sizes(&[vk::DescriptorPoolSize::builder().
						    type_(vk::DescriptorType::STORAGE_BUFFER).
						    descriptor_count(arr_args+1)]),
				   None)}?;
	println!("Also created the descriptor pool");
	defer!(this, ctx, ctx.dev.destroy_descriptor_pool(this.desc_pool, None));

	// allocate set
	this.desc_set = unsafe{ctx.dev.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder().
		descriptor_pool(this.desc_pool).
		set_layouts(&[this.desc_layout]))}?[0];
	println!("Allocated the single descriptor set from the pool");

	return Ok(this);
    }

    // Need to make arrays for this thing outside, and then use it here

    // Function to bind descriptors to buffers, then
    //  to write and execute on a command buffer
    // TODO:: Might need to decide if output has to be a reference
    pub fn record_cmd(&self,
		      inputs: &[DeviceF32Array],
		      output: DeviceF32Array,
		      scalar_args: &[f32]){
	let ctx = match self.ctx{
	    None => {return;},
	    Some(ctx) => ctx
	};
	let arg_arrs = [inputs, &[output]].concat();
	// Againfking rust
	let buf_infos:Vec<vk::DescriptorBufferInfoBuilder> = arg_arrs.iter().map(|arr|{
	    vk::DescriptorBufferInfo::builder().
		offset(0).
		range(arr.size as u64).
		buffer(arr.buffer)
	}).collect();
	let writes:Vec<vk::WriteDescriptorSetBuilder> =
	//arg_arrs.iter().enumerate().map(|(i, arr)|{
	    buf_infos.iter().enumerate().map(|(i, _)|{
		vk::WriteDescriptorSet::builder().
		    dst_set(self.desc_set).
		    dst_binding(i as u32).
		    dst_array_element(0).
		    descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
		    buffer_info(&buf_infos[i..(i+1)])
	    }).collect();
	unsafe{ctx.dev.update_descriptor_sets
	       (&writes,
		&([] as [vk::CopyDescriptorSet; 0]))};
	unsafe{ctx.dev.cmd_bind_descriptor_sets
	       (ctx.cmd_buff, vk::PipelineBindPoint::COMPUTE,
		self.pipe_layout, 0, &[self.desc_set], &([] as [u32;0]))};
	// Prepare the push constants and push!!!
	let sizes:Vec<u8> = arg_arrs.iter().flat_map(|x|{
	    (x.size as u32).to_ne_bytes().to_vec()
	}).collect();

	let sargs:Vec<u8> = scalar_args.iter().flat_map(|&x|{
	    x.to_ne_bytes().to_vec()
	}).collect();
	
	unsafe{ctx.dev.cmd_push_constants
	       (ctx.cmd_buff, self.pipe_layout, vk::ShaderStageFlags::COMPUTE, 0,
		&[sizes, sargs].concat())};

	unsafe{ctx.dev.cmd_bind_pipeline
	       (ctx.cmd_buff, vk::PipelineBindPoint::COMPUTE, self.pipeline)};
	// Find the optimum number of gpu to dispatch
	// TODO:: Later make it optimal
	let max_len = arg_arrs.iter().map(|x|{x.size}).max().unwrap_or(0);
	
	unsafe{ctx.dev.cmd_dispatch(ctx.cmd_buff, max_len as u32, 1, 1)};
    }
}

