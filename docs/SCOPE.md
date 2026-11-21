# EH3D-ISOCAM: scope

Status: **host-test**. EH3D-ISOCAM is a library, a command line and a window that turn the layers of a board into the paths, the G-code, the plots and the drill files that a milling machine needs to isolate the copper, drill the holes and cut the board out. It has been tried on a computer only, with the example files and with hand-written ones: no board has been milled with its paths. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The CAM program of the suite for printed circuit boards: it draws the copper on a grid, works out the exact distance from every cell to the copper, traces the paths where the tool has to go, and writes them as G-code, as an HP-GL plot for the Bungard machines or as a DXF drawing, together with the drill files, the cut round the board and the checks that tell a person what the tool cannot do.

## What it does

* Reads Gerber layers (also old-style layers when their apertures are given), drill files in mils or in millimetres, outlines from a Gerber layer, a DXF drawing or an HP-GL plot, and a CSV tool crib.
* Traces the isolation paths at the radius of the tool and the next passes one stepover farther each, with up to four tools, the leftover copper cleared with rings, forced isolation of narrow gaps and isolation of a single net.
* Writes the G-code (in millimetres or inches, with depth steps, a standard header and tool changes), HP-GL plots at 40 units to the millimetre and DXF drawings; follows the height map of AUTOLEVEL when it is asked to.
* Writes the drill files, a short path for the holes, the cut round the board with bridges, rounded corners and alignment holes, both sides of a board, panels and a solder-mask layer.
* Checks the design rules (copper too narrow, pieces too close, holes with too little copper), lists the places the tool cannot enter and refuses to write until they are looked at (or `--force`).
* Walks any G-code program move by move (`simulate`) and the program it is about to write, against the table and the limits of a machine, and refuses a cut that leaves the table with its line.
* Shows all of it in a window (built with `--features gui`), in eight languages, with a dry run and the path of the tool drawn over the board.

## What it will not do

* No decision about the limits of a machine, its interlocks or its emergency stop (see below).
* No Voronoi strategy, no fitting of arcs, no silkscreen layer yet (see `NOT_YET.md`).
* No sending of the program to a machine: it writes files.
* No claim that a board was milled with these paths: no board has been.
* No claim that the files it writes were read by other programs or machines: they have only been read back by the readers of this suite.

## Where it sits

* **It reads:** Gerber, drill, outline, DXF, HP-GL and CSV files, a height map, its settings file (`isocam.cfg`) and, when asked, the server of the suite.
* **It writes:** G-code, HP-GL plots, DXF drawings, drill files, Gerber layers (the solder mask), SVG previews and reports; never over a file it was made from, and when asked, the same on the server.
* **It is read by:** the machine-control programs of the suite and the Bungard routing program through those files, and the server; programs exchange work through files or through the server, never by depending on each other.
* **It depends on:** `EH3D-CORE-LIBS` (the readers and the raster and CAM crates), `EH3D-GERBERVIEWER` (the drawing of layers) and `EH3D-COMMON` (the Bungard formats and the client of the server), built next to it as sibling folders.

## What it never decides

The limits of a machine, its interlocks and its emergency stop belong to the machine and to its controller, not to this program. The check of the table and the limits of the machine can only make a program refused, and a program that passes is not a safe one: nothing here knows the stock, the clamps or the tool, and no board has been milled with these paths. Nothing here replaces the emergency stop of a machine or the judgement of the person who stands at it.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.
