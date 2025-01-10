#![allow(
    dead_code,
    unused_variables,
    unused_imports)]

use vulkanalia::VkResult;
use vulkanalia::vk;

mod context;
use context::Context;

mod process_unit;
// A struct for making `program` entities

fn main(){
    use vulkanalia::bytecode::Bytecode;
    use vulkanalia::prelude::v1_0::*;
    use vulkanalia::prelude::v1_1::*;
    use vulkanalia::prelude::v1_2::*;
    use vulkanalia::prelude::v1_3::*;
    let cxt = Context::new().unwrap();
    
    // Now include compute shader
    let comp_shader_code = Bytecode::new(include_bytes!("../shaders/add_arr.comp.spv")).unwrap();
    println!("The size of included shader is {}", comp_shader_code.code_size());

    let comp_shader_mod = unsafe{cxt.dev.
        create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().
                code_size(comp_shader_code.code_size()).
                code(comp_shader_code.code()),
            None).unwrap()};
    println!("Also created the compute shader module!");

    // Create the pipeline , then destroy the shader module
    let compute_info = [
        vk::ComputePipelineCreateInfo::builder().
            layout(cxt.pipe_layout).
            stage(vk::PipelineShaderStageCreateInfo::builder().
                stage(vk::ShaderStageFlags::COMPUTE).
                module(comp_shader_mod).
                name(b"main\0"))
    ];
    let compute_pipe = unsafe{cxt.dev.
        create_compute_pipelines(
            vk::PipelineCache::null(),
            &compute_info,
            None).unwrap()}.0[0];

    println!("Created the compute pipeline and then destroyed the shader module!");
    unsafe{cxt.dev.destroy_shader_module(comp_shader_mod, None)};

    // Create the descriptor pool and allocate/bind descriptors    
    let desc_pool = unsafe{cxt.dev.
        create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder().
                max_sets(2).
                pool_sizes(&[vk::DescriptorPoolSize::builder().
                    type_(vk::DescriptorType::STORAGE_BUFFER).
                    descriptor_count(2)]),
            None).unwrap()};
    
    println!("Also created the descriptor pool");

    // allocate set
    let desc_set = unsafe{cxt.dev.allocate_descriptor_sets(
        &vk::DescriptorSetAllocateInfo::builder().
            descriptor_pool(desc_pool).
            set_layouts(&[cxt.desc_layout])).unwrap()}[0];
    println!("Allocated the single descriptor set from the pool");

    // allocate buffers
    let arr_len:u32 = 32;
    let buff1 = cxt.new_array(arr_len as usize, false).unwrap();
    let buff2 = cxt.new_array(arr_len as usize, false).unwrap();
    println!("The memory index chosen for buffers is {}", cxt.vis_buff_mem_type);
    
    println!("Memories for buffer were allocated!");

    // Bind descriptors to buffers
    unsafe{cxt.dev.update_descriptor_sets(&[
        vk::WriteDescriptorSet::builder().
	    dst_set(desc_set).
            dst_binding(0).
            dst_array_element(0).
            descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
            buffer_info(&[vk::DescriptorBufferInfo::builder().
                offset(0).
                range(buff1.size as u64).
                buffer(buff1.buffer)]),
        vk::WriteDescriptorSet::builder().
	    dst_set(desc_set).
            dst_binding(1).
            dst_array_element(0).
            descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
            buffer_info(&[vk::DescriptorBufferInfo::builder().
                offset(0).
                range(buff2.size as u64).
                buffer(buff2.buffer)])],
        &([] as [vk::CopyDescriptorSet; 0]))};
    println!("The descriptors were written onto with buffers");
    
    // Command buffer recording
    unsafe{cxt.dev.begin_command_buffer(cxt.cmd_buff, &vk::CommandBufferBeginInfo::builder()).unwrap()};
    unsafe{cxt.dev.cmd_bind_descriptor_sets(cxt.cmd_buff, vk::PipelineBindPoint::COMPUTE,
        cxt.pipe_layout, 0, &[desc_set], &([] as [u32;0]))};
    unsafe{cxt.dev.cmd_push_constants(cxt.cmd_buff, cxt.pipe_layout, vk::ShaderStageFlags::COMPUTE, 0, &arr_len.to_ne_bytes())};
    unsafe{cxt.dev.cmd_push_constants(cxt.cmd_buff, cxt.pipe_layout, vk::ShaderStageFlags::COMPUTE, 4, &arr_len.to_ne_bytes())};
    unsafe{cxt.dev.cmd_bind_pipeline(cxt.cmd_buff, vk::PipelineBindPoint::COMPUTE, compute_pipe)};
    unsafe{cxt.dev.cmd_dispatch(cxt.cmd_buff, arr_len, 1, 1)};
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
    unsafe{cxt.dev.destroy_descriptor_pool(desc_pool, None)};
    unsafe{cxt.dev.destroy_pipeline(compute_pipe, None)};
}
