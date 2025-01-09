#![allow(
    dead_code,
    unused_variables,
    unused_imports)]

use vulkanalia::vk::*;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::prelude::v1_1::*;
use vulkanalia::prelude::v1_2::*;
use vulkanalia::prelude::v1_3::*;

const VALIDATION_ENABLED: bool =
    cfg!(debug_assertions);

const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation\0");

fn write_to_c_pointer(ptr: *mut u8, values: &[f32]) {
    unsafe {
        // Ensure the pointer is aligned correctly for f32
        let aligned_ptr = ptr as *mut f32;

        // Check alignment (f32 requires 4-byte alignment)
        if (aligned_ptr as usize) % std::mem::align_of::<f32>() != 0 {
            panic!("Pointer is not properly aligned for f32.");
        }

        // Use memcpy-like behavior (copy bytes from values into the allocated memory)
        std::ptr::copy_nonoverlapping(values.as_ptr() as *const u8, aligned_ptr as *mut u8, values.len() * std::mem::size_of::<f32>());
    }
}

fn read_from_c_pointer(ptr: *mut u8, num_elements: usize) -> Vec<f32> {
    unsafe {
        // Ensure the pointer is aligned correctly for f32
        let aligned_ptr = ptr as *mut f32;

        // Check alignment (f32 requires 4-byte alignment)
        if (aligned_ptr as usize) % std::mem::align_of::<f32>() != 0 {
            panic!("Pointer is not properly aligned for f32.");
        }

        // Create a slice from the raw pointer (reinterpret the pointer)
        let slice = std::slice::from_raw_parts_mut(aligned_ptr, num_elements);
        
        // Return the slice as a Vec
        slice.to_vec()
    }
}

fn main(){
    println!("Hello, world!");

    let loader = unsafe{LibloadingLoader::new(LIBRARY).unwrap()};
    let entry = unsafe{Entry::new(loader).unwrap()};

    let app_info = ApplicationInfo::builder().
        application_name(b"Jpt Application\0").
        application_version(vk::make_version(1,0,0)).
        engine_name(b"Null Engine\0").
        engine_version(vk::make_version(1,0,0)).
        api_version(vk::make_version(1,3,0));
    // TODO:: Check if layers/extensions are available first
    let inst = unsafe{entry.
        create_instance(&InstanceCreateInfo::builder().
            application_info(&app_info).
            enabled_layer_names(&vec![VALIDATION_LAYER.as_ptr()]),
            None).unwrap()};
    println!("Yes, the instance was created!!");

    //find the physical device
    let phy_devs = unsafe{inst.enumerate_physical_devices().unwrap()};

    for (i, dev) in phy_devs.iter().enumerate(){
        let dev_props = unsafe{inst.get_physical_device_properties(*dev)};
        println!("Device number {} is {}", i, dev_props.device_name);
    }
    // For now choose literally the first option

    // TODO::setup at least the validation layer and it's extensions

    // find the compute queue family
    let queue_fam_props = unsafe{inst.get_physical_device_queue_family_properties(phy_devs[0])};
    let compute_fam = {
        let mut fam_inx:i32 = -1;
        for (i, fam) in queue_fam_props.iter().enumerate(){
            if QueueFlags::empty() != (fam.queue_flags & QueueFlags::COMPUTE) {
                fam_inx = i as i32;
                break;
            }
        }
        if fam_inx < 0{
            panic!("Compute family index was not found!");
        }
        fam_inx as u32
    };
    println!("Compute family index was found for the selected physical device to be at index {}.", compute_fam);


    let queue_priorities = [1.0f32];
    let queue_infos = [DeviceQueueCreateInfo::builder().
        queue_family_index(compute_fam).
        queue_priorities(&queue_priorities)];

    let dev = unsafe{inst.
        create_device(
            phy_devs[0],
            &DeviceCreateInfo::builder().
                queue_create_infos(&queue_infos),
            None).unwrap()};

    println!("The logical device was also created successfully!");

    let compute_queue = unsafe{dev.get_device_queue(compute_fam, 0)};
    println!("Got the compute queue object!");
    // Now allocate a command pool and buffer
    
    let cmd_pool = unsafe{dev.
        create_command_pool(&CommandPoolCreateInfo::builder().
            queue_family_index(compute_fam).
            flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
            None).unwrap()};
    println!("Created the command pool also!");

    let cmd_buf = unsafe{dev.
        allocate_command_buffers(&CommandBufferAllocateInfo::builder().
            level(CommandBufferLevel::PRIMARY).
            command_buffer_count(1).
            command_pool(cmd_pool)).unwrap()}[0];
    println!("Command buffer allocated!");

    // Now include compute shader
    let comp_shader_code = Bytecode::new(include_bytes!("../shaders/add_arr.comp.spv")).unwrap();
    println!("The size of included shader is {}", comp_shader_code.code_size());

    let comp_shader_mod = unsafe{dev.
        create_shader_module(
            &ShaderModuleCreateInfo::builder().
                code_size(comp_shader_code.code_size()).
                code(comp_shader_code.code()),
            None).unwrap()};
    println!("Also created the compute shader module!");

    // Create descriptor set layout/ pipeline layout
    let desc_layout = unsafe{dev.create_descriptor_set_layout
        (&DescriptorSetLayoutCreateInfo::builder().
            // TODO:: Find if different types of descriptors need to be set for readonly/readwrite glsl buffers
            bindings(&[
                DescriptorSetLayoutBinding::builder().
                    binding(0).
                    descriptor_type(DescriptorType::STORAGE_BUFFER).
                    descriptor_count(1).
                    stage_flags(ShaderStageFlags::COMPUTE),
                DescriptorSetLayoutBinding::builder().
                    binding(1).
                    descriptor_type(DescriptorType::STORAGE_BUFFER).
                    descriptor_count(1).
                    stage_flags(ShaderStageFlags::COMPUTE)]),
            None).unwrap()};
    println!("Descriptor set layout was created!");

    let pipe_layout = unsafe{dev.
        create_pipeline_layout(
            &PipelineLayoutCreateInfo::builder().
                set_layouts(&[desc_layout]).
                push_constant_ranges(&[PushConstantRange::builder().
                    stage_flags(ShaderStageFlags::COMPUTE).
                    offset(0).
                    size(4)]),
            None).unwrap()};
    println!("The pipeline layout was created!");

    // Create the pipeline layout, then destroy the shader module
    let compute_info = [
        ComputePipelineCreateInfo::builder().
            layout(pipe_layout).
            stage(PipelineShaderStageCreateInfo::builder().
                stage(ShaderStageFlags::COMPUTE).
                module(comp_shader_mod).
                name(b"main\0"))
    ];
    let compute_pipe = unsafe{dev.
        create_compute_pipelines(
            PipelineCache::null(),
            &compute_info,
            None).unwrap()}.0[0];

    println!("Created the compute pipeline and then destroyed the shader module!");
    unsafe{dev.destroy_shader_module(comp_shader_mod, None)};

    // Create the descriptor pool and allocate/bind descriptors    
    let desc_pool = unsafe{dev.
        create_descriptor_pool(
            &DescriptorPoolCreateInfo::builder().
                max_sets(2).
                pool_sizes(&[DescriptorPoolSize::builder().
                    type_(DescriptorType::STORAGE_BUFFER).
                    descriptor_count(2)]),
            None).unwrap()};
    
    println!("Also created the descriptor pool");

    // allocate set
    let desc_set = unsafe{dev.allocate_descriptor_sets(
        &DescriptorSetAllocateInfo::builder().
            descriptor_pool(desc_pool).
            set_layouts(&[desc_layout])).unwrap()}[0];
    println!("Allocated the single descriptor set from the pool");

    // allocate buffers
    let arr_len:u32 = 32;
    let buff_size:u64 = (arr_len as u64) * 4;
    let buff1 = unsafe{dev.create_buffer(&BufferCreateInfo::builder().
        usage(BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::TRANSFER_SRC).
        sharing_mode(SharingMode::EXCLUSIVE).
        size(buff_size),
        None).unwrap()};
    let buff2 = unsafe{dev.create_buffer(&BufferCreateInfo::builder().
        usage(BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::TRANSFER_SRC).
        sharing_mode(SharingMode::EXCLUSIVE).
        size(buff_size),
        None).unwrap()};
    println!("Created input and output buffer objects");
    
    // Find memory type index, one requirement works for both
    let buff_mem_req = unsafe{dev.get_buffer_memory_requirements(buff1)};
    println!("The buffers have memory type bits {} and size {} and alignment {}",
        buff_mem_req.memory_type_bits, buff_mem_req.size, buff_mem_req.alignment);

    let buff_mem_inx = {
        // TODO:: Later get the atomic transfer size if needed from device properties
        let dev_mem_props = unsafe{inst.get_physical_device_memory_properties(phy_devs[0])};
        let mem_type_flags = buff_mem_req.memory_type_bits;
        let mem_props = MemoryPropertyFlags::DEVICE_LOCAL | MemoryPropertyFlags::HOST_VISIBLE;
        let mut mem_inx : i32 = -1;
        for inx in 0..dev_mem_props.memory_type_count as usize{
            let is_type = ((1<<inx) & mem_type_flags) != 0;
            let is_props = mem_props == (mem_props &
                dev_mem_props.memory_types[inx].property_flags);
            if is_type && is_props{
                mem_inx = inx as i32;
                break;
            }
        }
        if mem_inx < 0{
            panic!("A suitable memory type was not found for the buffers!");
        }
        mem_inx as u32
    };
    println!("The memory index chosen for buffers is {}", buff_mem_inx);
    
    // Allocate the memory for both buffers (maybe separately)
    let buff1_vk_mem = unsafe{dev.allocate_memory(
        &MemoryAllocateInfo::builder().
            allocation_size(buff_size).
            memory_type_index(buff_mem_inx),
        None).unwrap()};
    let buff2_vk_mem = unsafe{dev.allocate_memory(
        &MemoryAllocateInfo::builder().
            allocation_size(buff_size).
            memory_type_index(buff_mem_inx),
        None).unwrap()};
    println!("Memories for buffer were allocated!");

    unsafe{dev.bind_buffer_memory(buff1, buff1_vk_mem, 0).unwrap()};
    unsafe{dev.bind_buffer_memory(buff2, buff2_vk_mem, 0).unwrap()};
    println!("The buffers were bound to respective memories");

    // Bind descriptors to buffers
    unsafe{dev.update_descriptor_sets(&[
        WriteDescriptorSet::builder().
	    dst_set(desc_set).
            dst_binding(0).
            dst_array_element(0).
            descriptor_type(DescriptorType::STORAGE_BUFFER).
            buffer_info(&[DescriptorBufferInfo::builder().
                offset(0).
                range(buff_size).
                buffer(buff1)]),
        WriteDescriptorSet::builder().
	    dst_set(desc_set).
            dst_binding(1).
            dst_array_element(0).
            descriptor_type(DescriptorType::STORAGE_BUFFER).
            buffer_info(&[DescriptorBufferInfo::builder().
                offset(0).
                range(buff_size).
                buffer(buff2)])],
        &([] as [CopyDescriptorSet; 0]))};
    println!("The descriptors were written onto with buffers");
    
    // Command buffer recording
    unsafe{dev.begin_command_buffer(cmd_buf, &CommandBufferBeginInfo::builder()).unwrap()};
    unsafe{dev.cmd_bind_descriptor_sets(cmd_buf, PipelineBindPoint::COMPUTE,
        pipe_layout, 0, &[desc_set], &([] as [u32;0]))};
    unsafe{dev.cmd_push_constants(cmd_buf, pipe_layout, ShaderStageFlags::COMPUTE, 0, &arr_len.to_ne_bytes())};
    unsafe{dev.cmd_bind_pipeline(cmd_buf, PipelineBindPoint::COMPUTE, compute_pipe)};
    unsafe{dev.cmd_dispatch(cmd_buf, arr_len, 1, 1)};
    unsafe{dev.end_command_buffer(cmd_buf).unwrap()};
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
        let mem_map = unsafe{dev.map_memory(buff1_vk_mem, 0, buff_size, MemoryMapFlags::empty()).unwrap()};
        println!("Mapped memory to {}", mem_map as u64);

        write_to_c_pointer(mem_map as *mut u8, &input_data);
        unsafe{dev.unmap_memory(buff1_vk_mem)};
    }
    // Print the data too
    {
        let mem_map = unsafe{dev.map_memory(buff1_vk_mem, 0, buff_size, MemoryMapFlags::empty()).unwrap()};
        println!("Mapped memory to {}", mem_map as u64);

        let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        println!("The array was \n{:?}", arr);
        unsafe{dev.unmap_memory(buff1_vk_mem)};
    }   
    
    // Play command buffer
    unsafe{dev.queue_submit(compute_queue,
        &[SubmitInfo::builder().command_buffers(&[cmd_buf])],
        Fence::null()).unwrap()};
    println!("Played the command buffer");
    // Wait
    unsafe{dev.device_wait_idle().unwrap()};
    
    // Print data
    {
        let mem_map = unsafe{dev.map_memory(buff2_vk_mem, 0, buff_size, MemoryMapFlags::empty()).unwrap()};
        println!("Mapped the answer memory to {}", mem_map as u64);

        let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        println!("The answer was \n{:?}", arr);
        unsafe{dev.unmap_memory(buff2_vk_mem)};
    }   
    // Print the data too
    {
        let mem_map = unsafe{dev.map_memory(buff1_vk_mem, 0, buff_size, MemoryMapFlags::empty()).unwrap()};
        println!("Mapped original memory to {}", mem_map as u64);

        let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        println!("The original array is \n{:?}", arr);
        unsafe{dev.unmap_memory(buff1_vk_mem)};
    }   
    

    unsafe{dev.device_wait_idle().unwrap()};
    unsafe{dev.free_memory(buff2_vk_mem, None)};
    unsafe{dev.free_memory(buff1_vk_mem, None)};
    unsafe{dev.destroy_buffer(buff2, None)};
    unsafe{dev.destroy_buffer(buff1, None)};
    unsafe{dev.destroy_descriptor_pool(desc_pool, None)};
    unsafe{dev.destroy_pipeline(compute_pipe, None)};
    unsafe{dev.destroy_pipeline_layout(pipe_layout, None)};
    unsafe{dev.destroy_descriptor_set_layout(desc_layout, None)};
    unsafe{dev.destroy_command_pool(cmd_pool, None)};
    unsafe{dev.destroy_device(None)};
    unsafe{inst.destroy_instance(None)};
}
