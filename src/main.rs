mod cups;

#[tokio::main]
async fn main() {
    let path = "/mnt/c/Users/User/Desktop/Kozachenko CV(current).txt";

    println!("Print result: {:?}", cups::print_pdf(path));
}
