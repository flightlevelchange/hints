use cfg_if::cfg_if;

fn main() {
    cfg_if! {
        if #[cfg(feature = "standalone")] {
            hints::run_standalone();
        } else {
            println!("This program requires the 'standalone' feature to be enabled.");
        }
    }
}
