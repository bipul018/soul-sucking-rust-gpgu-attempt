# An Attempt at Making a General Purpose Computing Mechanism Library in rust on top of vulkan

This is a small example trial that tries to use vulkanalia crate's minimal vulkan abstractions and the minimum of vulkan extensions to make a simple program that executes the following:
y = 2x
z = 3y
x = z

for a few number of times.

It consists of just 3 units, 
+ a context that stores vulkan instance, device, queue, command buffers.
+ a 'factory' like object consisting of a compute pipeline instance
+ any number of instances produced using the factory that just have to satisfy some traits, and forms the actual 'computation node'. It consists of the 'after pipeline creation' information like length of the arrays, output arrays and any scalar inputs to be given to the computation node used internally as push constants. 


I tried and tried but failed to utilize rust's type system generics and traits to guarentee the precious safety that rustaceans are always preaching about. 
So I gave up and used just runtime guarantees as much as possible, but still the code as a whole is not that 'safe' looking only from compile time.
Barely a example managed to run without errors, both from rust and vulkan's validation layers (warnings in the validation layers is a different story).

Finally decided to give up for a while as I found some other GPGPU libraries on vulkan in C++ that actually matched my agendas when I was doing this project.

