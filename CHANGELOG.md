# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.1] - Isolation milling of copper from a Gerber layer: exact offsets, several passes and gap checks

- **The copper on a grid:** pads, tracks and regions drawn on cells of the chosen size, with a limit on the memory it may take.
- **The exact distance:** the Euclidean distance from every cell to the copper (checked against a brute force), and the paths of the passes traced where it equals the radius of the tool, one stepover farther for each further pass.
- **The gap check:** pieces of copper that are closer than the tool is wide are listed; `isolate` refuses to write unless `--force` is given.
- **The G-code:** lifts between paths, plunges to the depth chosen and cuts the nearest path next; an output file that ends in `.plt` gets an HP-GL plot file with one pen for each pass, for the routing program of the Bungard machines.
- **The command line** `eh3d-isocam` with `isolate` and `check`, and a sample layer.
- **17 tests** run on a computer, with damaged input.
