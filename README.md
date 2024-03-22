# beacon_ww3_eject_simulator

This repo simulates the effects of the inactivity leak on big networks. Initially I attempted to run this on a jupyter notebook, but for 1M indices each simulation took 1 hour 🫠. Rust is a bit faster to do math, taking ~45 seconds per simulation. Plotting sucks, but it's okay.

### `EJECTION_BALANCE = 16`

| inactive % | inactivity_leak_stop_days | % total_balance_burned |
| - | - | - |
| 35 | 7.0 | 3.3 |
| 40 | 13.8 | 12.7 |
| 50 | 21.4 | 30.1 |
| 60 | 26.6 | 45.2 |
| 70 | 31.4 | 59.6 |
| 80 | 36.5 | 73.3 |
| 90 | 43.12 | 86.2 |

### `EJECTION_BALANCE = 31.99` (immediate)

| inactive % | inactivity_leak_stop_days | % total_balance_burned |
| - | - | - |
| 35 | 4.7 | 1.5 |
| 40 | 11.3 | 8.4 |
| 50 | 19.2 | 23.9 |
| 60 | 25.1 | 39.1 |
| 70 | 30.3 | 53.7 |
| 80 | 35.7 | 67.5 |
| 90 | 42.4 | 80.5 |

### `EJECTION_BALANCE = 0` (no ejection)

| inactive % | inactivity_leak_stop_days | % total_balance_burned |
| - | - | - |
| 35 | 7.0 | 3.3 |
| 40 | 13.8 | 12.7 |
| 50 | 21.4 | 30.2 |
| 60 | 27.0 | 46.1 |
| 70 | 32.0 | 61.0 |
| 80 | 37.1 | 75.0 |
| 90 | 43.8 | 88.1 |
