use core::panic::PanicInfo;
use crate::console::kprintln;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {

    kprintln!("
            (
       (      )     )
         )   (    (
        (          `
    .-\"\"^\"\"\"^\"\"^\"\"\"^\"\"-.
  (//\\\\//\\\\//\\\\//\\\\//\\\\//)
   ~\\^^^^^^^^^^^^^^^^^^/~
     `================`

    The pi is overdone.
");
    if let Some(location) = _info.location() {
	kprintln!("FILE: {}\n LINE: {}\n COL: {}\n", location.file(), location.line(), location.column());
    }

    kprintln!("{:?}", _info);
    
    
    loop {}
}
