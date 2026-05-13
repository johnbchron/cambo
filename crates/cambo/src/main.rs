use portable_pty::{CommandBuilder, PtySize, native_pty_system};

fn main() {
  let pty_system = native_pty_system();

  let pair = pty_system
    .openpty(PtySize {
      rows:         24,
      cols:         80,
      pixel_width:  0,
      pixel_height: 0,
    })
    .expect("failed to open pty");

  let cmd = CommandBuilder::new("bash");
  let _child = pair
    .slave
    .spawn_command(cmd)
    .expect("failed to open bash process on pty child side");

  let mut reader = pair
    .master
    .try_clone_reader()
    .expect("failed to get read handle to pty master side");
  let mut writer = pair
    .master
    .take_writer()
    .expect("failed to get write handle to master side");

  writeln!(writer, "ls -l\n").expect("failed to write to master write handle");

  let mut stdout = std::io::stdout();
  std::io::copy(&mut reader, &mut stdout)
    .expect("failed to copy bytes from pty reader to stdout");
}
