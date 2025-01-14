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
use vulkanalia::vk::ExtDebugUtilsExtension;

// Make a struct, that setups everything except for the pipeline descriptorpool/set/buffer/memory

#[derive(Debug)]
pub struct Context{
    pub entry: vulkanalia::Entry,

    #[cfg(debug_assertions)]
    pub messenger: vk::DebugUtilsMessengerEXT,


    pub inst: vulkanalia::Instance,
    pub phy_dev: vk::PhysicalDevice,
    pub dev: vulkanalia::Device,
    pub comp_fam: u32,

    cleanup: Vec<fn(&mut Context)>,
    pub comp_queue: vk::Queue,
    pub cmd_pool: vk::CommandPool,
    pub cmd_buff: vk::CommandBuffer,
    pub copy_cmd_buff: vk::CommandBuffer,

    pub vis_buff_mem_type: u32, // Type index used for cpu visible memory types (for now also device local considering compatibility with my device only
    pub loc_buff_mem_type: u32, // Type index used for gpu local memory types
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

use std::ffi::c_void;
use std::ffi::CStr;

#[cfg(debug_assertions)]
extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        panic!("ERROR: ({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        println!("WARNING: ({:?}) {}", type_, message);
    } else {
        if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
            //println!("INFO: ({:?}) {}", type_, message);
        } else {
            //println!("TRACE: ({:?}) {}", type_, message);
        }
    }

    vk::FALSE
}

impl Context{
    pub fn new() -> VkResult<Context>{

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

	    let mut layers = Vec::new();
	    let mut extensions = Vec::new();

	    if VALIDATION_ENABLED {
		layers.push(VALIDATION_LAYER.as_ptr());
		extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
	    }
            let mut inst_info = vk::InstanceCreateInfo::builder().
		application_info(&app_info).
		enabled_layer_names(&layers).
                enabled_extension_names(&extensions);

            // Enable extra validation features if debug mode


            let mut validation_features =  vk::ValidationFeaturesEXT::builder().
                enabled_validation_features(&[
                    vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
                    // TODO:: Fix why this is not working yet
                    //vk::ValidationFeatureEnableEXT::DEBUG_PRINTF,
                    vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
                    //vk::ValidationFeatureEnableEXT::GPU_ASSISTED,
                ]);

            
            let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .user_callback(Some(debug_callback));
            if VALIDATION_ENABLED {
                inst_info = inst_info.push_next(&mut validation_features);
                inst_info = inst_info.push_next(&mut debug_info);
            }

            let inst = unsafe{entry.
                create_instance(&inst_info, None)?};
            println!("Yes, the instance was created !!");
            return Ok((entry,inst));
        }

        #[cfg(debug_assertions)]
        fn new_messenger(inst: vulkanalia::Instance) -> (vulkanalia::Instance, vk::DebugUtilsMessengerEXT){
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                //.flags(vk::DebugUtilsMessengerCreateFlagsEXT)
                .message_type(
                    //vk::DebugUtilsMessageTypeFlagsEXT::all()
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                )
                .user_callback(Some(debug_callback));
            let messenger = unsafe{ inst.create_debug_utils_messenger_ext(&debug_info, None) }.unwrap();
            return (inst, messenger);
        }

        fn new_device(inst: vulkanalia::Instance) -> VkResult<(vulkanalia::Instance, vulkanalia::Device, vk::PhysicalDevice, u32)>{
            println!("Going to create the gpu device ...");
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

            let dev = unsafe{match inst.
                create_device(
                    phy_devs[0],
                    &vk::DeviceCreateInfo::builder().
                        queue_create_infos(&queue_infos).
			push_next(&mut vk::PhysicalDeviceVulkan13Features::builder()
				  .synchronization2(true)),
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
        // Create messenger only on debug, on debug it will crash if fail
        #[cfg(debug_assertions)]
        let (inst, messenger) = new_messenger(inst);
        let (inst, dev, phy_dev, comp_fam) = new_device(inst)?;
        let mut this = {
            macro_rules! def{
                () => {Default::default()}
            }
            Context{
                entry, inst, dev, phy_dev, comp_fam,
                #[cfg(debug_assertions)]
messenger,

                cleanup: def!(),
                comp_queue: def!(),
                cmd_pool: def!(),
                cmd_buff: def!(),
		copy_cmd_buff: def!(),
                loc_buff_mem_type: def!(),
                vis_buff_mem_type: def!(),
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
            }
        }
        defer!(s, s.inst.destroy_instance(None));
        #[cfg(debug_assertions)]
        defer!(s, s.inst.destroy_debug_utils_messenger_ext(s.messenger, None));
        defer!(s, s.dev.destroy_device(None));
        
        this.comp_queue = unsafe{this.dev.get_device_queue(this.comp_fam, 0)};
        println!("Got the compute queue object!");
        // Now allocate a command pool and buffer
    
        this.cmd_pool = unsafe{this.dev.
            create_command_pool(&vk::CommandPoolCreateInfo::builder().
                queue_family_index(this.comp_fam).
                flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None)?};
        println!("Created the command pool also!");
        defer!(s, s.dev.destroy_command_pool(s.cmd_pool, None));

        //this.cmd_buff = unsafe{this.dev.
	let cmd_buffs = unsafe{this.dev.
            allocate_command_buffers(&vk::CommandBufferAllocateInfo::builder().
                level(vk::CommandBufferLevel::PRIMARY).
                command_buffer_count(2).
                command_pool(this.cmd_pool))?};
        println!("Command buffers were allocated!");
	this.cmd_buff = cmd_buffs[0];
	this.copy_cmd_buff = cmd_buffs[1];
	

        // Find the memory types by creating dummy buffers
        {
            let tbuff = unsafe{this.dev.create_buffer(&vk::BufferCreateInfo::builder().
                usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC).
                sharing_mode(vk::SharingMode::EXCLUSIVE).
                size(8 * 4),
                None)?};
            let buff_mem_req = unsafe{this.dev.get_buffer_memory_requirements(tbuff)};
            // TODO:: Later get the atomic transfer size if needed from device properties
            let dev_mem_props = unsafe{this.inst.get_physical_device_memory_properties(this.phy_dev)};
            let mem_type_flags = buff_mem_req.memory_type_bits;
            // For device local memory
            this.loc_buff_mem_type = {
                let mem_props = vk::MemoryPropertyFlags::DEVICE_LOCAL;
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
                    // TODO:: Need to return a proper error instead of panicking
                    panic!("A suitable memory type was not found for the device local buffers!");
                }
                mem_inx as u32
            };

            // For host visible memory
            this.vis_buff_mem_type = {
                //TODO:: Need to make this just host visible later
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
                    // TODO:: Need to return a proper error instead of panicking
                    panic!("A suitable memory type was not found for the host visible buffers!");
                }
                mem_inx as u32
            };

            unsafe{this.dev.destroy_buffer(tbuff, None)};

        }

        return Ok(this);
    }

    // Function that returns allocated memory for f32 array bounded with buffer
    pub fn new_array(&self, count: usize, is_dev_local: bool) -> VkResult<DeviceF32Array>{
        let buff_size:u64 = (count as u64) * 4;
        let buff = unsafe{self.dev.create_buffer(&vk::BufferCreateInfo::builder().
            usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC).
            sharing_mode(vk::SharingMode::EXCLUSIVE).
            size(buff_size),
            None)?};
        // TODO:: Decide if need to check this array being of same memory type or not
        if is_dev_local{
            unimplemented!("Currently wont support device local only things as other fxns are not robust enough for using this type of objects");
        }
        let vk_mem = unsafe{self.dev.allocate_memory(
            &vk::MemoryAllocateInfo::builder().
                allocation_size(buff_size).
                memory_type_index(if is_dev_local {self.loc_buff_mem_type} else {self.vis_buff_mem_type}),
            None)};
        // Free buff if vk_mem failed
        let vk_mem = match vk_mem{
            Ok(ok) => Ok(ok),
            Err(err) => {
                unsafe{self.dev.destroy_buffer(buff, None)};
                Err(err)
            }
        }?;
        // Free buff and vk_mem if binding failed
        match unsafe{self.dev.bind_buffer_memory(buff, vk_mem, 0)}{
            Ok(_) => {},
            Err(err) => {
                unsafe{self.dev.destroy_buffer(buff, None)};
                unsafe{self.dev.free_memory(vk_mem, None)};
                return Err(err);
            }
        }
        return Ok(DeviceF32Array{
            buffer: buff,
            memory: vk_mem,
            is_dev_local,
            count,
            size: (buff_size as usize),
        });
    }
    // Function that copies the given float values into the gpu representing memory
    pub fn write_array(&self, array: &DeviceF32Array, values: &[f32]) {
        let buff_size = (array.count * 4) as u64;
        let mem_map = unsafe{self.dev.map_memory(array.memory, 0, buff_size, vk::MemoryMapFlags::empty()).unwrap()};
        // Ensure the pointer is aligned correctly for f32
        let ptr = mem_map as *mut f32;
        // TODO:: Check for length of values being right
        unsafe {
            //let aligned_ptr = ptr as *mut f32;

            // Check alignment (f32 requires 4-byte alignment)
            if (ptr as usize) % std::mem::align_of::<f32>() != 0 {
                panic!("Pointer is not properly aligned for f32.");
            }

            // Use memcpy-like behavior (copy bytes from values into the allocated memory)
            std::ptr::copy_nonoverlapping(values.as_ptr() as *const u8, ptr as *mut u8, values.len() * std::mem::size_of::<f32>());
        }
        unsafe{self.dev.unmap_memory(array.memory)};
    }
    // Function that copies the given float values from the gpu representing memory
    pub fn read_array(&self, array: &DeviceF32Array) -> Vec<f32> {
        let buff_size = (array.count * 4) as u64;
        let mem_map = unsafe{self.dev.map_memory(array.memory, 0, buff_size, vk::MemoryMapFlags::empty()).unwrap()};
        // Ensure the pointer is aligned correctly for f32
        let ptr = mem_map as *mut f32;

        let v = unsafe {
            //let aligned_ptr = ptr as *mut f32;

            // Check alignment (f32 requires 4-byte alignment)
            if (ptr as usize) % std::mem::align_of::<f32>() != 0 {
                panic!("Pointer is not properly aligned for f32.");
            }

            // Create a slice from the raw pointer (reinterpret the pointer)
            let slice = std::slice::from_raw_parts_mut(ptr, array.count);
        
            // Return the slice as a Vec
            slice.to_vec()
        };
        unsafe{self.dev.unmap_memory(array.memory)};
        return v;
    }
    // Function that frees a array
    pub fn drop_array(&self, array: &DeviceF32Array) {
        unsafe{self.dev.destroy_buffer(array.buffer, None)};
        unsafe{self.dev.free_memory(array.memory, None)};
    }
}
// Doesnot feel that right doing a default derive on this, but fk rust
#[derive(Copy, Clone, Debug, Default)]
pub struct DeviceF32Array{
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub is_dev_local: bool,
    pub count: usize, // count is in f32 units
    pub size: usize
}

