mod cups;
mod cups_ffi;

#[tokio::main]
async fn main() {
    let path = "/mnt/c/Users/User/Desktop/Kozachenko CV(current).pdf";
    // let path = "/mnt/c/Users/User/Desktop/Kozachenko CV(current).txt";

    // cups::simple_print("Title", path);

    unsafe {
        cups::print_pdf(path);
    }
}
