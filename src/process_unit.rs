use vulkanalia::VkResult;
use vulkanalia::vk;

//mod context;
use crate::context::Context;
use crate::context::DeviceF32Array;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::prelude::v1_1::*;
use vulkanalia::prelude::v1_2::*;
use vulkanalia::prelude::v1_3::*;

#[derive(Default)]
pub struct FactoryObjectBase<'a, 'b>{
    desc_pool: vk::DescriptorPool,
    desc_set: vk::DescriptorSet,
    //Needed because at writing of command buffer, need to set barriers (TODO:: Insert mode for when array is copied also)
    // TODO:: Also need to find out if inserting barriers will disturb when we are doing it after copying in same command submission
    // Second item is the expected size of array, need to be initialized at init time
    // TODO:: See if you can replace these vecs with some arrays with sizes, since these need not be modified once created 
    inputs: Vec<(Option<&'b DeviceF32Array>, u32)>,
    //inputs: Vec<Option<&'b DeviceF32Array>>,
    
    outputs: Vec<DeviceF32Array>,
    scalars: Vec<u8>,

    // Fk rust, i cant just have 'all other except this default'
    factory: Option<&'a Factory<'a>>,
    //ctx: Option<&'a Context>,
}

impl Drop for FactoryObjectBase<'_, '_>{
    fn drop(&mut self){
        self.clean();
    }
}

impl<'a,'b> FactoryObjectBase<'a,'b>{
    pub fn ctx(&self) -> Option<&'a Context>{ self.factory?.ctx }
    pub fn clean(&mut self){
	match self.ctx(){
	    None => {},
	    Some(ctx) => {
                // Only cleanup if not null for each
                self.outputs.map(|&arr|{
                    ctx.drop_array(arr);
                    arr = DeviceF32Array::default();
                });
                self.inputs.map(|&oarr|{
                    oarr = (None, 0);
                });
                if self.desc_pool != vk::DescriptorPool::null(){
                    ctx.dev.destroy_descriptor_pool(self.desc_pool);
                    self.desc_pool = vk::DescriptorPool::null();
                }
                self.desc_set = vk::DescriptorSet::null();
	    }
	}
    }
    pub fn new(factory: &'a Factory, input_sizes: &[u32], output_sizes: &[u32], scalar_count: usize) -> VkResult<Self>{
        let mut this = Self::default();
        this.factory = factory;
        //TODO:: Print a informative error
        let ctx = this.ctx().unwrap();

        this.desc_pool = unsafe{ctx.dev.
	    create_descriptor_pool(
		&vk::DescriptorPoolCreateInfo::builder().
		    max_sets(1).
		    pool_sizes(&[vk::DescriptorPoolSize::builder().
			type_(vk::DescriptorType::STORAGE_BUFFER).
			descriptor_count((input_sizes.len() + output_sizes.len()) as u32)]),
		None)}?;
        
	// allocate set
	this.desc_set = unsafe{ctx.dev.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder().
		descriptor_pool(this.desc_pool).
		set_layouts(&[factory.desc_layout]))}?[0];

        this.inputs = input_sizes.map(|sz|{ (None, sz as u32) });
        // 4 byts each for u32 or f32 or i32
        this.scalars = vec![0; scalar_count * 4];
        // Use a push mechanism for allowing cleanup rather than map
        for (inx, sz) in output_sizes.into_iter().enumerate(){
            let arr = ctx.new_array(sz as usize, false)?;
            this.outputs.push(arr);
            unsafe{ctx.dev.update_descriptor_sets
            (&[vk::WriteDescriptorSet::builder().
                dst_set(this.desc_set).
                dst_binding((input_sizes.len() + inx) as u32).
                dst_array_element(0).
                descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
                buffer_info(&[vk::DescriptorBufferInfo::builder().
                    offset(0).
                    range(arr.size).
                    buffer(arr.buffer)])],
                &([] as [vk::CopyDescriptorSet; 0]))};
        }
        return Ok(this);
    }
    // function to set a input array (also on descriptor)
    pub fn write_input(&mut self, inx: usize, arr_ref: &'b DeviceF32Array){
        let ctx = self.ctx().unwrap(); // Should we signal error ??
        // Zero, assert that the length supplied in is matching
        assert_eq!(arr_ref.len() as u32, self.inputs[inx].1,
            "Length of supplied array is different from the expected value of length given at construction");
        
        // First write to the array list
        self.inputs[inx].0 = arr_ref;
        // Then write to descriptor assuming initialized
        unsafe{ctx.dev.update_descriptor_sets
            (&[vk::WriteDescriptorSet::builder().
                dst_set(self.desc_set).
                dst_binding(inx as u32).
                dst_array_element(0).
                descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
                buffer_info(&[vk::DescriptorBufferInfo::builder().
                    offset(0).
                    range(arr_ref.size).
                    buffer(arr_ref.buffer)])],
                &([] as [vk::CopyDescriptorSet; 0]))};        
    }

    // function to set a push constant (scalar)
    pub fn write_scalar(&mut self, inx: usize, scalar_val:ScalarArgVal){
        let ctx = self.ctx().unwrap(); // Should we signal error ??
        // Size of a push constant is of either u32 or f32
        let vec_inx = inx * 4;
        self.scalars[vec_inx..(vec_inx+4)].
            copy_from_slice(&scalar_val.to_ne_vec());
    }
    // function to get a output
    pub fn get_output(&self, inx: usize) -> &DeviceF32Array{
        return &self.outputs[inx];
    }

    // function to write to command buffer with barriers for inputs only and push constants and also binds pipeline except for the dispatches??
    // To make it able to work with multiple dispatches at once 
    pub fn setup_pre_cmd(&self, cmd_buf: &vk::CommandBuffer) {
        // TODO:: Make it return error instead of asserting
        let fact = match self.fact{
            None => assert!(false, "Need to have been produced from a factory. Homemade goods are not upto standard"),
            Some(fact) => fact
        };
        let ctx = fact.ctx.unwrap(); // Should we signal error ??

        // Write the barriers
        // TODO:: Later allow to use a FactoryObject kind / copy operation kind, etc to distinguish between buffer that doesnot need any barrier, vs buffer that needs barriers against copy operation, vs buffer that needs barriers against write operation
        let buff_barrs:Vec<vk::BufferMemoryBarrier2Builder> = 
            self.inputs.map(|(x,_)| {
                let arr_ref = match x{
                    None => assert!(x != None, "All inputs must have been set before executing command"),
                    Some(a) => a,
                };
                vk::BufferMemoryBarrier2::builder().
                    src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER).
                    dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER).
                    src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE).
                    dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ).
                    src_queue_family_index(ctx.comp_fam).
                    dst_queue_family_index(ctx.comp_fam).
                    buffer(arr_ref.buffer).
                    offset(0).
                    size(arr_ref.size)
            });
        unsafe{ctx.dev.cmd_pipeline_barrier2(
            cmd_buf,
            &vk::DependencyInfo::builder().
                dependency_flags(vk::DependencyFlags::empty()).
                buffer_memory_barriers(&buff_barrs)
        )};
        unsafe{ctx.dev.cmd_push_constants
                (cmd_buf, fact.pipe_layout, vk::ShaderStageFlags::COMPUTE, 0, &self.scalars)};
	unsafe{ctx.dev.cmd_bind_pipeline
	       (cmd_buf, vk::PipelineBindPoint::COMPUTE, fact.pipeline)};
    }
}


pub trait FactoryObject{
    // These should be same for all instances
    const INPUT_ARRAY_COUNT: usize;
    const INPUT_SCALAR_COUNT: usize;
    const OUTPUT_ARRAY_COUNT: usize;
    const INPUT_SCALAR_TYPES: &'static [ScalarArgType];


    // A static fxn that takes in the 'knob' struct and produces valuable information
    type Knobs;
    // Becauze fking rust is a fking dumb little shit
    type Array<N> = [u32;N];
    fn input_array_sizes(knobs: Self::Knobs) ->
Self::Array<Self::INPUT_ARRAY_COUNT>;
//[u32;Self::INPUT_ARRAY_COUNT];
    fn output_array_sizes(knobs: Self::Knobs) ->
Self::Array<Self::OUTPUT_ARRAY_COUNT>;
//[u32;Self::OUTPUT_ARRAY_COUNT];

    // Generate factory using the generic parameter as self
    // Provides the shader itself along with anything needed
    // This line happened because of fking rust. I cannot (in ways that a normal user should be able to do) ensure that Factory made with a type in mind only be used for that type. Fking have to use unsafe rust now. I swear after this basic version is completed i am quite likely quitting rust for good no matter if even god tells me to.
    //fn factory<'a>(ctx: &'a Context) -> VkResult<Factory<'a, Self>>;
    fn factory<'a>(ctx: &'a Context) -> VkResult<Factory<'a>>;
    fn new(base_obj: FactoryObjectBase, knobs: Self::Knobs) -> VkResult<Self> where Self: Sized;

    // Usage fxns
    // TODO:: Maybe it will be nicer to have a single fxn that takes in all needed arguments through another struct (like the Knobs) where one specifies all the necessary input arguments (like compulsory named parameters) and a command buffer and then it does everything like setting the arguments and calling setup_pre_cmd for the base object and then calling the dispatch. But for now will just have separate fxn that does everything separately ??
    type Inputs;
    fn exec_cmd(&mut self, cmd_buf: &vk::CommandBuffer, args: Self::Inputs);
}


pub struct Factory<'a>{
    pub desc_layout: vk::DescriptorSetLayout,
    pub pipe_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,

    // F*CK RUST, FU*K RUST, FUC* RUST, *UCK RUST
    // If I try to uncomment this below line, then i have to literally litter the whole code base with additional generic type T and still it won't compile at a point telling me nonsense "it cannot know the size of type at compile time" like wtf is the use of std::marker::PhantomData then. I just wanted to make sure that this factory object wouldnot be misused by intitializing for a type and producing for another type
    //_marker_for_assoc_type_of_product: std::marker::PhantomData<T>,
    ctx: Option<&'a Context>,
}
impl Drop for Factory<'_>{
    fn drop(&mut self){
        self.clean();
    }
}
impl<'a, T> Factory<'a> where T: FactoryObject{
    pub fn clean(&mut self){
        match self.ctx{
	    None => {},
	    Some(ctx) => {
                // Only cleanup if not null for each
                if self.pipeline != vk::Pipeline::null(){
                    unsafe{ctx.dev.destroy_pipeline(self.pipeline, None)};
                    self.pipeline = None;
                }
                if self.pipe_layout != vk::PipelineLayout::null(){
                    unsafe{ctx.dev.destroy_pipeline_layout(self.pipe_layout, None)};
                    self.pipe_layout = vk::DescriptorSetLayout::null();
                }
                if self.desc_layout != vk::DescriptorSetLayout::null(){
                    unsafe{ctx.dev.destroy_descriptor_set_layout(self.desc_layout, None)};
                    self.desc_layout = vk::DescriptorSetLayout::null();
                }
	    }
	}
    }

    pub fn new(ctx: &'a Context, shader_code: &Bytecode) -> VkResult<Factory<'a>>{

        let mut this = Self::default();
	this.ctx = Some(ctx);

        let desc_bindings:Vec<vk::DescriptorSetLayoutBindingBuilder> = (0..(T::INPUT_ARRAY_COUNT+T::OUTPUT_ARRAY_COUNT)).map(|i|{
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
         
        this.pipe_layout = unsafe{ctx.dev.
            create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder().
                    set_layouts(&[this.desc_layout]).
                    push_constant_ranges(&[vk::PushConstantRange::builder().
                        stage_flags(vk::ShaderStageFlags::COMPUTE).
                        offset(0).
                        size(4 * T::INPUT_SCALAR_COUNT as u32)]),
                None)?};

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
        
	return Ok(this);
    }

    // Will also take additional struct type that is directly forwarded to type T as arguments
    pub fn produce(&self, knobs: T::Knobs) -> VkResult<T> {
        // Assert that we do have context
        let ctx = match self.ctx{
            None => assert!(false, "Need to have context to produce anything"),
            Some(c) => c,
        };
        
        // first make the factory object base
        let base_obj = FactoryObjectBase::new(ctx, self.desc_layout, &T::input_array_sizes(knobs), &T::output_array_sizes(knobs), T::INPUT_SCALAR_COUNT)?;
        // Then pass it into the factory object's new function
        return T::new(base_obj, knobs);
    }
    

}


// Fking rust is annoying i hate it i hate it i hate it i hate it
#[derive(Clone,Debug,PartialEq, Copy)]
pub enum ScalarArgType{
    ArrayLen,
    U32,
    F32
}
#[derive(Clone,Debug,Copy)]
pub enum ScalarArgVal<'a>{
    ArrayLen(&'a DeviceF32Array),
    U32(u32),
    F32(f32)
}
impl ScalarArgVal<'_>{
    pub fn to_ne_vec(&self) -> Vec<u8>{
        match self{
            Self::U32(v) => v.to_ne_bytes(),
            Self::F32(v) => v.to_ne_bytes(),
            Self::ArrayLen(arr_ref) => (arr_ref.size as u32).to_ne_bytes(),
        }.to_vec()
    }
}

pub fn scalar_arg_type_ok(p_type: ScalarArgType,
    p_val: &ScalarArgVal) -> bool
{
    match p_val {
        ScalarArgVal::ArrayLen(_) => p_type == ScalarArgType::ArrayLen,
        ScalarArgVal::U32(_) => p_type == ScalarArgType::U32,
        ScalarArgVal::F32(_) => p_type == ScalarArgType::F32,
    }
}
