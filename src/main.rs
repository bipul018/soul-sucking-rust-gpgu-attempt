#![allow(
    dead_code,
    unused_variables,
    unused_imports)]

use vulkanalia::VkResult;
use vulkanalia::vk;
const VALIDATION_ENABLED: bool =
    cfg!(debug_assertions);

use vulkanalia::prelude::v1_0::*;
use vulkanalia::prelude::v1_1::*;
use vulkanalia::prelude::v1_2::*;
use vulkanalia::prelude::v1_3::*;
const VALIDATION_LAYER: vk::ExtensionName = vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation\0");

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

// Make a struct, that setups everything except for the pipeline descriptorpool/set/buffer/memory

struct Context{
    entry: vulkanalia::Entry,
    inst: vulkanalia::Instance,
    phy_dev: vk::PhysicalDevice,
    dev: vulkanalia::Device,
    comp_fam: u32,

    cleanup: Vec<fn(&mut Context)>,
    comp_queue: vk::Queue,
    cmd_pool: vk::CommandPool,
    cmd_buff: vk::CommandBuffer,
    desc_layout: vk::DescriptorSetLayout,
    pipe_layout: vk::PipelineLayout,
}
impl Drop for Context{
    fn drop(&mut self){
        // First, collect the cleanup functions in reverse order
        let cleanup_functions: Vec<fn(&mut Context)> =
            self.cleanup.clone().into_iter().rev().collect();

        // Now, call each cleanup function with a mutable reference to `self`
        for cleanup_fn in cleanup_functions {
            cleanup_fn(self);
        }        
    }
}

impl Context{
    fn new() -> VkResult<Context>{

        use vulkanalia::bytecode::Bytecode;
        use vulkanalia::prelude::v1_0;
        use vulkanalia::prelude::v1_1;
        use vulkanalia::prelude::v1_2;
        use vulkanalia::prelude::v1_3;

        use vulkanalia::loader::{LibloadingLoader, LIBRARY};

        fn new_instance() -> VkResult<(vulkanalia::Entry, vulkanalia::Instance)>{
            // TODO:: Fix this libloading error to ? instead of unwrap
            let loader = unsafe{LibloadingLoader::new(LIBRARY).unwrap()};
            println!("Hello, world!");
            let entry = unsafe{vulkanalia::Entry::new(loader).unwrap()};
            let app_info = vk::ApplicationInfo::builder().
                application_name(b"Jpt Application\0").
                application_version(vk::make_version(1,0,0)).
                engine_name(b"Null Engine\0").
                engine_version(vk::make_version(1,0,0)).
                api_version(vk::make_version(1,3,0));
            // TODO:: Check if layers/extensions are available first

            let inst = unsafe{entry.
                create_instance(&vk::InstanceCreateInfo::builder().
                    application_info(&app_info).
                    enabled_layer_names(&vec![VALIDATION_LAYER.as_ptr()]),
                    None)?};
            println!("Yes, the instance was created !!");
            return Ok((entry,inst));
        }

        // A option of instance wrapper and option of device wrapper that use drop
        //struct ODevice{
        //    inst: vulkanalia::Instance,
        //    dev: vulkanalia::Device,
        //    phy_dev: vk::PhysicalDevice,
        //    comp_fam: u32,
        //}
        //fn new_device(inst: vulkanalia::Instance) -> VkResult<ODevice>{
        fn new_device(inst: vulkanalia::Instance) -> VkResult<(vulkanalia::Instance, vulkanalia::Device, vk::PhysicalDevice, u32)>{
        //fn new_device() -> VkResult<(vulkanalia::Instance, vulkanalia::Device, vk::PhysicalDevice, u32)>{
            //let (entry,inst) = new_instance()?;
            println!("Going to create the gpu device ...");
            //let mut this = ODevice::default();
            //find the physical device
            let phy_devs = unsafe{match inst.enumerate_physical_devices(){
                Ok(ok) => Ok(ok),
                Err(err) => {
                    inst.destroy_instance(None);
                    Err(err)
                }
            }}?;
            println!("Found some physical devices");
            for (i, dev) in phy_devs.iter().enumerate(){
                let dev_props = unsafe{inst.get_physical_device_properties(*dev)};
                println!("Device number {} is {}", i, dev_props.device_name);
            }
            // For now choose literally the first option
            let phy_dev = phy_devs[0];

            // TODO::setup at least the validation layer and it's extensions

            // find the compute queue family

            //let compute_fam = {
            let comp_fam = {
                let queue_fam_props = unsafe{inst.get_physical_device_queue_family_properties(phy_dev)};
                let mut fam_inx:i32 = -1;
                for (i, fam) in queue_fam_props.iter().enumerate(){
                    if vk::QueueFlags::empty() != (fam.queue_flags & vk::QueueFlags::COMPUTE) {
                        fam_inx = i as i32;
                        break;
                    }
                }
                if fam_inx < 0{
                    // TODO:: Return a compatible error here
                    panic!("Compute family index was not found!");
                }
                fam_inx as u32
            };
            println!("Compute family index was found for the selected physical device to be at index {}.", comp_fam);


            let queue_priorities = [1.0f32];
            let queue_infos = [vk::DeviceQueueCreateInfo::builder().
                queue_family_index(comp_fam).
                queue_priorities(&queue_priorities)];

            //let dev = unsafe{inst.
            let dev = unsafe{match inst.
                create_device(
                    phy_devs[0],
                    &vk::DeviceCreateInfo::builder().
                        queue_create_infos(&queue_infos),
                    None){
                        Ok(ok) => Ok(ok),
                        Err(err) => {
                            inst.destroy_instance(None);
                            Err(err)
                        }
                    }}?;
            println!("The logical device was also created successfully!");
            return Ok((inst, dev, phy_dev, comp_fam));
        }

        // new_device() will have destroyed instance on error
        let (entry, inst) = new_instance()?;
        let (inst, dev, phy_dev, comp_fam) = new_device(inst)?;
        let mut this = {
            macro_rules! def{
                () => {Default::default()}
            }
            Context{
                entry, inst, dev, phy_dev, comp_fam,
                cleanup: def!(),
                comp_queue: def!(),
                cmd_pool: def!(),
                cmd_buff: def!(),
                desc_layout: def!(),
                pipe_layout: def!(),
            }
        };

        macro_rules! defer{
            ($iden: ident, $expr: expr) => {
                this.cleanup.push(
                    |$iden|{
                        unsafe{
                            $expr;
                        }
                    }
                );
                //this.cleanup.push(|s| { unsafe{$expr} });
            }
        }
        defer!(s, s.inst.destroy_instance(None));
        defer!(s, s.dev.destroy_device(None));
        
        this.comp_queue = unsafe{this.dev.get_device_queue(this.comp_fam, 0)};
        println!("Got the compute queue object!");
        // Now allocate a command pool and buffer
    
        this.cmd_pool = unsafe{this.dev.
            create_command_pool(&vk::CommandPoolCreateInfo::builder().
                queue_family_index(this.comp_fam).
                flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None).unwrap()};
        println!("Created the command pool also!");
        defer!(s, s.dev.destroy_command_pool(s.cmd_pool, None));

        this.cmd_buff = unsafe{this.dev.
            allocate_command_buffers(&vk::CommandBufferAllocateInfo::builder().
                level(vk::CommandBufferLevel::PRIMARY).
                command_buffer_count(1).
                command_pool(this.cmd_pool)).unwrap()}[0];
        println!("Command buffer allocated!");

        // Create descriptor set layout/ pipeline layout
        this.desc_layout = unsafe{this.dev.create_descriptor_set_layout
        (&vk::DescriptorSetLayoutCreateInfo::builder().
            // TODO:: Find if different types of descriptors need to be set for readonly/readwrite glsl buffers
            bindings(&[
                vk::DescriptorSetLayoutBinding::builder().
                    binding(0).
                    descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
                    descriptor_count(1).
                    stage_flags(vk::ShaderStageFlags::COMPUTE),
                vk::DescriptorSetLayoutBinding::builder().
                    binding(1).
                    descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
                    descriptor_count(1).
                    stage_flags(vk::ShaderStageFlags::COMPUTE)]),
            None).unwrap()};
        println!("Descriptor set layout was created!");


        defer!(s, s.dev.destroy_descriptor_set_layout(s.desc_layout, None));
        
        this.pipe_layout = unsafe{this.dev.
            create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder().
                    set_layouts(&[this.desc_layout]).
                    push_constant_ranges(&[vk::PushConstantRange::builder().
                        stage_flags(vk::ShaderStageFlags::COMPUTE).
                        offset(0).
                        size(4)]),
                None).unwrap()};
        println!("The pipeline layout was created!");
        defer!(s, s.dev.destroy_pipeline_layout(s.pipe_layout, None));

        return Ok(this);
    }
}

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
    let buff_size:u64 = (arr_len as u64) * 4;
    let buff1 = unsafe{cxt.dev.create_buffer(&vk::BufferCreateInfo::builder().
        usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC).
        sharing_mode(vk::SharingMode::EXCLUSIVE).
        size(buff_size),
        None).unwrap()};
    let buff2 = unsafe{cxt.dev.create_buffer(&vk::BufferCreateInfo::builder().
        usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC).
        sharing_mode(vk::SharingMode::EXCLUSIVE).
        size(buff_size),
        None).unwrap()};
    println!("Created input and output buffer objects");
    
    // Find memory type index, one requirement works for both
    let buff_mem_req = unsafe{cxt.dev.get_buffer_memory_requirements(buff1)};
    println!("The buffers have memory type bits {} and size {} and alignment {}",
        buff_mem_req.memory_type_bits, buff_mem_req.size, buff_mem_req.alignment);

    let buff_mem_inx = {
        // TODO:: Later get the atomic transfer size if needed from device properties
        let dev_mem_props = unsafe{cxt.inst.get_physical_device_memory_properties(cxt.phy_dev)};
        let mem_type_flags = buff_mem_req.memory_type_bits;
        let mem_props = vk::MemoryPropertyFlags::DEVICE_LOCAL | vk::MemoryPropertyFlags::HOST_VISIBLE;
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
    let buff1_vk_mem = unsafe{cxt.dev.allocate_memory(
        &vk::MemoryAllocateInfo::builder().
            allocation_size(buff_size).
            memory_type_index(buff_mem_inx),
        None).unwrap()};
    let buff2_vk_mem = unsafe{cxt.dev.allocate_memory(
        &vk::MemoryAllocateInfo::builder().
            allocation_size(buff_size).
            memory_type_index(buff_mem_inx),
        None).unwrap()};
    println!("Memories for buffer were allocated!");

    unsafe{cxt.dev.bind_buffer_memory(buff1, buff1_vk_mem, 0).unwrap()};
    unsafe{cxt.dev.bind_buffer_memory(buff2, buff2_vk_mem, 0).unwrap()};
    println!("The buffers were bound to respective memories");

    // Bind descriptors to buffers
    unsafe{cxt.dev.update_descriptor_sets(&[
        vk::WriteDescriptorSet::builder().
	    dst_set(desc_set).
            dst_binding(0).
            dst_array_element(0).
            descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
            buffer_info(&[vk::DescriptorBufferInfo::builder().
                offset(0).
                range(buff_size).
                buffer(buff1)]),
        vk::WriteDescriptorSet::builder().
	    dst_set(desc_set).
            dst_binding(1).
            dst_array_element(0).
            descriptor_type(vk::DescriptorType::STORAGE_BUFFER).
            buffer_info(&[vk::DescriptorBufferInfo::builder().
                offset(0).
                range(buff_size).
                buffer(buff2)])],
        &([] as [vk::CopyDescriptorSet; 0]))};
    println!("The descriptors were written onto with buffers");
    
    // Command buffer recording
    unsafe{cxt.dev.begin_command_buffer(cxt.cmd_buff, &vk::CommandBufferBeginInfo::builder()).unwrap()};
    unsafe{cxt.dev.cmd_bind_descriptor_sets(cxt.cmd_buff, vk::PipelineBindPoint::COMPUTE,
        cxt.pipe_layout, 0, &[desc_set], &([] as [u32;0]))};
    unsafe{cxt.dev.cmd_push_constants(cxt.cmd_buff, cxt.pipe_layout, vk::ShaderStageFlags::COMPUTE, 0, &arr_len.to_ne_bytes())};
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
        let mem_map = unsafe{cxt.dev.map_memory(buff1_vk_mem, 0, buff_size, vk::MemoryMapFlags::empty()).unwrap()};
        println!("Mapped memory to {}", mem_map as u64);

        write_to_c_pointer(mem_map as *mut u8, &input_data);
        unsafe{cxt.dev.unmap_memory(buff1_vk_mem)};
    }
    // Print the data too
    {
        let mem_map = unsafe{cxt.dev.map_memory(buff1_vk_mem, 0, buff_size, vk::MemoryMapFlags::empty()).unwrap()};
        println!("Mapped memory to {}", mem_map as u64);

        let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        println!("The array was \n{:?}", arr);
        unsafe{cxt.dev.unmap_memory(buff1_vk_mem)};
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
        let mem_map = unsafe{cxt.dev.map_memory(buff2_vk_mem, 0, buff_size, vk::MemoryMapFlags::empty()).unwrap()};
        println!("Mapped the answer memory to {}", mem_map as u64);

        let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        println!("The answer was \n{:?}", arr);
        unsafe{cxt.dev.unmap_memory(buff2_vk_mem)};
    }   
    // Print the data too
    {
        let mem_map = unsafe{cxt.dev.map_memory(buff1_vk_mem, 0, buff_size, vk::MemoryMapFlags::empty()).unwrap()};
        println!("Mapped original memory to {}", mem_map as u64);

        let arr = read_from_c_pointer(mem_map as *mut u8, arr_len as usize);
        println!("The original array is \n{:?}", arr);
        unsafe{cxt.dev.unmap_memory(buff1_vk_mem)};
    }   
    

    unsafe{cxt.dev.device_wait_idle().unwrap()};
    unsafe{cxt.dev.free_memory(buff2_vk_mem, None)};
    unsafe{cxt.dev.free_memory(buff1_vk_mem, None)};
    unsafe{cxt.dev.destroy_buffer(buff2, None)};
    unsafe{cxt.dev.destroy_buffer(buff1, None)};
    unsafe{cxt.dev.destroy_descriptor_pool(desc_pool, None)};
    unsafe{cxt.dev.destroy_pipeline(compute_pipe, None)};
    //unsafe{dev.destroy_pipeline_layout(cxt.pipe_layout, None)};
    //unsafe{dev.destroy_descriptor_set_layout(desc_layout, None)};
    //unsafe{dev.destroy_command_pool(cmd_pool, None)};
    //unsafe{dev.destroy_device(None)};
    //unsafe{inst.destroy_instance(None)};
}
