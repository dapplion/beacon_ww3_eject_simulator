# beacon_ww3_eject_simulator

This repo simulates the effects of the inactivity leak on big networks. Initially I attempted to run this on a jupyter notebook, but for 1M indices each simulation took 1 hour 🫠. Rust is a bit faster to do math, taking ~45 seconds per simulation. Plotting sucks, but it's okay.

Some preliminary results:

| Percent of validators nuked | Time to finality (days) | Avg balance of nuked validators (ETH) |
| - | - | - |
| 35 | 7.0  | 28.9
| 40 | 13.8 | 21.8
| 50 | 21.4 | 12.7
| 60 | 26.6 | 7.91
| 70 | 31.4 | 4.75
| 80 | 36.5 | 2.67
| 90 | 43.1 | 1.32
