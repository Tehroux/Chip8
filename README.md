# Chip8 emulator

The emulator run at 500hz, but sprite are rendered at 60hz.
This make the emulator feel smoother due to the draw command waiting.

## not implemented
- sound timer
- wait key is on pressed and not released

## Test
The emulator was tested with rom from [https://github.com/Timendus/chip8-test-suite](https://github.com/Timendus/chip8-test-suite)

## screenshot
### Display chip8 logo
![chip8 logo](screenshots/test_rom_1.png)

### Display IBM logo
![IBM logo](screenshots/test_rom_2.png)

### Test instruction
Rom that test instruction implementation
![Instruction test](screenshots/test_instruction.png)

### Test flags
Rom that test flags logic
![flags test](screenshots/test_flag.png)

### Test quirks
Rom that test implementation quirks

![quirk test](screenshots/test_quirk_1.png)
![quirk test](screenshots/test_quirk_2.png)
![quirk test](screenshots/test_quirk_3.png)

### Test key
Rom that test key pressed. The third test fail since my implementation
of get_key wait for key pressed and not key released.

![key test](screenshots/test_key_1.png)
![key test](screenshots/test_key_2.png)
