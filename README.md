# i3 Ratio Splitter

When I split windows in i3, I almost always end up resizing the new window down to ~33% of the width or height of the screen, having one main window and one reference window in the same workspace. I then want future tiles to split in the opposite direction, and resize similarly. This is a little utility that automatically resizes newly created windows, using i3's ipc mechanism.

### Configuration

i3-ratiosplit will look for a configuration file in `~/.config/i3/ratiosplit.ini`. Possible options are (defaults are filled in below):

```ini
[main]
ratio = 0.33
log_file_level = info # off, error, warn, info, debug, trace
log_file = "~/.config/i3/ratiosplit.log"
log_console_level = off # off, error, warn, info, debug, trace
```
