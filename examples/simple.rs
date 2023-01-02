use sync_cow::SyncCow;

fn main() {
    let cow = SyncCow::new(5);

    let val = cow.read();
    println!("Val: {}", *val);
    cow.edit(|x| *x = 9);
    println!("Old val after write: {}", *val);
    println!("New val after write: {}", *cow.read());
}
