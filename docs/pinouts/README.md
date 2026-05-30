# Board pinouts

Downloaded references for the boards currently supported by this project.

## WeAct STM32F411CEU6 BlackPill

- `weact_blackpill_stm32f411_pinout.png` - quick visual pinout.
- `weact_blackpill_stm32f4x1_pin_layout.pdf` - official WeAct pin layout PDF.

Source:
- https://github.com/WeActStudio/WeActStudio.MiniSTM32F4x1

## NUCLEO-H743ZI2

- `nucleo_h743zi2_zio_left.png` - Zio header, top left side.
- `nucleo_h743zi2_zio_right.png` - Zio header, top right side.
- `nucleo_h743zi2_morpho_left.png` - CN11/CN12 morpho header, left side.
- `nucleo_h743zi2_morpho_right.png` - CN11/CN12 morpho header, right side.
- `nucleo_h743zi2_pinout_legend.png` - legend for the pinout colors.

Current H7 project wiring is configured in `src/pinmap.rs` and kept on
Arduino/Zio headers:

| Project signal | Arduino header | MCU pin | Timer |
|---|---|---|---|
| Light-show LED | D6 | PE9 | TIM1_CH1 |
| Voice 0 PWM | A0 | PA3 | TIM2_CH4 |
| Voice 1 PWM | D0 | PB7 | TIM4_CH2 |

Reserved for a typical SPI display shield:

| Display signal | Arduino header | MCU pin |
|---|---|---|
| SPI SCK | D13 | PA5 |
| SPI MISO | D12 | PA6 |
| SPI MOSI | D11 | PB5 |
| Display CS | D10 | PD14 |
| Display DC | D9 | PD15 |
| Display reset | D8 | PF3 |

Sources:
- https://os.mbed.com/platforms/ST-Nucleo-H743ZI2/
- https://www.st.com/resource/en/user_manual/um2407-stm32h7-nucleo144-board-stmicroelectronics.pdf

Note: the ST UM2407 PDF is the official full manual with connector tables.
The local ST download timed out from PowerShell, so keep the official link here
for the full manual and use the downloaded PNGs for quick wiring.
