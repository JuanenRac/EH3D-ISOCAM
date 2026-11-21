# EH3D-ISOCAM: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because no board has been milled with these paths and nothing of this program has run on a real machine.

The register has 45 requirements: 34 implemented, 4 prototype, 7 planned and 0 validated. By progress: 34 done, 4 partial, 4 pending and 3 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `ISOC-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against a simulated machine (the walk of the G-code over the table of the machine) or a server in memory, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `file::name` is the test `name` of `tests/file.rs` (`src/file::name` is a test inside `src/file.rs`), which must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## Isolation paths

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ISOC-001 | The distance from every cell to the nearest copper is exact. | implemented | done | host-test | `isocam::the_distance_transform_is_exact` | - |
| ISOC-002 | The first pass is one tool radius from the copper and more passes go one stepover farther each. | implemented | done | host-test | `isocam::more_passes_go_one_stepover_farther_each` | - |
| ISOC-003 | The tool never gets closer to the copper than its radius, and the simplified path stays within the tolerance. | implemented | done | host-test | `isocam::the_tool_never_gets_closer_to_the_copper_than_its_radius` | Exact to half a cell, not to the curve itself. |
| ISOC-004 | Two pieces of copper closer than the tool are reported, and the G-code is not written until they are looked at. | implemented | done | host-test | `isocam::two_pieces_closer_than_the_tool_are_reported_and_distant_ones_are_not` | - |
| ISOC-005 | A path is laid along the middle of a gap the tool does not fit in, only when forced isolation is on. | implemented | done | host-test | `advanced::a_path_is_laid_along_the_middle_of_a_gap_the_tool_does_not_fit_in` | A forced path lets a tool wider than the gap touch the copper on both sides. |
| ISOC-006 | The widest tool cuts first and where it has cut the finer tool does not go again. | implemented | done | host-test | `multi::where_the_wide_tool_has_cut_the_fine_tool_does_not_go_again` | - |
| ISOC-007 | Rings of a wider tool clear the leftover copper and keep off the copper. | implemented | done | host-test | `multi::the_rings_of_leftover_copper_overlap_and_keep_off_the_copper` | Places narrower than the tool keep their copper. |
| ISOC-008 | Isolating one net cuts round that net alone. | implemented | done | host-test | `problems::isolate_with_a_net_cuts_round_that_net_alone_with_the_net_tool` | - |
| ISOC-009 | Garbage layers do not panic. | implemented | done | host-test | `isocam::garbage_layers_do_not_panic` | - |

## Files written

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ISOC-010 | The G-code cuts each closed path at the depth, lifts between paths and stops for the tool only when its width changes. | implemented | done | host-test | `formats::the_program_stops_for_the_tool_only_when_the_width_changes` | - |
| ISOC-011 | The same passes give the same G-code every time, and a number never says minus zero. | implemented | done | host-test | `formats::the_same_passes_give_the_same_gcode_every_time` | - |
| ISOC-012 | The G-code of the passes has the standard header and no alert when it is read back. | implemented | done | host-test | `generated_gcode::the_command_line_starts_the_gcode_with_the_standard_header_and_the_program_has_no_alert` | Read back with the checker of the suite, not run on a machine. |
| ISOC-013 | The passes are written as HP-GL with one pen for each pass at forty units to the millimetre. | implemented | done | host-test | `isocam::the_passes_as_hpgl_use_one_pen_for_each_pass_and_forty_units_to_the_millimetre` | Read back by the reader of this suite, not by the routing program of the machines. |
| ISOC-014 | The passes are written as a DXF drawing with a layer for each pass. | implemented | done | host-test | `formats::the_dxf_has_a_layer_for_each_pass_a_flag_for_closed_paths_and_all_the_vertices` | - |
| ISOC-015 | Each cut is made again at each depth with steps of depth, and an open run is cut back the way it came. | implemented | done | host-test | `depth_and_levelling::an_open_run_is_cut_back_the_way_it_came_at_the_next_depth` | - |
| ISOC-016 | The Z of every cut follows the height map of AUTOLEVEL, in pieces of at most the set length. | implemented | done | host-test | `depth_and_levelling::the_cut_follows_the_surface_in_pieces_no_longer_than_asked` | Only maps from the program of the suite or hand-written ones. |
| ISOC-017 | Inches convert every length and the feeds, and mils are refused. | implemented | done | host-test | `formats::inches_convert_every_length_and_the_feeds_but_not_the_speed_and_mils_are_refused` | - |

## Holes, outline and layout

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ISOC-018 | A drill file in mils is read with its tools, and a metric one with the reader of the suite. | implemented | done | host-test | `files::a_bungard_drill_file_in_mils_is_read_with_its_tools` | - |
| ISOC-019 | Drills written are read back the same, and damaged drill files are errors and never panic. | implemented | done | host-test | `files::drills_written_are_read_back_the_same` | Only read back by the readers of this suite. |
| ISOC-020 | The order of the holes is kept unless the settings ask for a short path, and the travel is told. | implemented | done | host-test | `files::the_drill_travel_is_told_and_the_start_of_the_tool_counts` | - |
| ISOC-021 | The tool goes round the board outside and inside the cutouts. | implemented | done | host-test | `files::the_tool_goes_round_the_board_outside_and_inside_the_cutouts` | - |
| ISOC-022 | Bridges open the cut into runs with gaps and can be kept off the corners. | implemented | done | host-test | `files::bridges_are_moved_off_the_corners_when_a_clearance_is_asked_for` | - |
| ISOC-023 | An outline is read from a Gerber layer, a DXF or an HP-GL plot, and strokes drawn one by one join into a closed outline. | implemented | done | host-test | `files::an_outline_is_read_from_gerber_and_from_a_plotter_file` | - |
| ISOC-024 | Turning a board over twice gives it back, and the alignment holes are where the board finds them turned over. | implemented | done | host-test | `layout::the_alignment_holes_are_where_the_board_finds_them_after_it_is_turned_over` | - |
| ISOC-025 | A panel repeats the board with a gap. | implemented | done | host-test | `layout::a_panel_repeats_the_board_with_a_gap` | - |
| ISOC-026 | The corners of the cut round the board can be rounded, and both settings are off by default. | implemented | done | host-test | `outline_shape::rounding_the_corners_shortens_the_cut_and_a_cutout_that_is_left_out_leaves_no_path` | - |
| ISOC-027 | The solder-mask openings are written as a Gerber layer that reads back. | implemented | done | host-test | `advanced::the_mask_is_written_as_a_gerber_layer_that_reads_back` | Follows the pads as drawn; tracks stay covered. |

## Checks and limits of the machine

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ISOC-028 | Copper narrower than the rule and copper closer than the rule are reported where they are. | implemented | done | host-test | `layout::copper_closer_than_the_rule_is_reported_where_it_is` | Rules do not check against the board edge or the solder mask. |
| ISOC-029 | A sharper corner is reported as copper left in its tip, and a right angle is not. | implemented | done | host-test | `problems::a_narrow_notch_is_reported_and_a_right_angle_is_not` | - |
| ISOC-030 | A board larger than the table is refused with the line and nothing is written. | implemented | done | host-test | `job_safety::a_board_larger_than_the_table_is_refused_with_the_line_and_nothing_is_written` | - |
| ISOC-031 | A simulated machine halts at the first move out of the table. | prototype | partial | simulator | `machine_travel::a_board_that_does_not_fit_the_smaller_table_halts_the_machine_at_the_first_move_out` | A model made for the tests, not a real machine. |
| ISOC-032 | The dry run of a program has a time, a box and its findings, and the limits of the machine apply. | implemented | done | host-test | `job_safety::the_dry_run_applies_the_limits_of_the_machine` | The time is estimated from the speeds, not measured. |
| ISOC-033 | The offset of the work moves every output by the same amount and nothing else. | implemented | done | host-test | `offset::the_paths_of_the_copper_move_by_the_offset_and_by_nothing_else` | - |

## Crib, server and window

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ISOC-034 | The tool of the crib sets the width an end mill or a V-bit cuts with at the depth, and a tool that is too thin is refused. | implemented | done | host-test | `crib::the_command_line_takes_the_tool_from_the_crib_and_tells_it` | Read from the CSV table, not from the JSON file of the crib. |
| ISOC-035 | The output is kept on the server with the values it was made with, and nothing is sent without an address. | prototype | partial | simulator | `cli_reading::the_output_of_isolate_is_kept_on_the_server_with_the_values_it_was_made_with` | Tried against a server in memory, not against a real server. |
| ISOC-036 | The window opens the sample, works out the paths and refuses a value the job cannot use. | prototype | partial | host-test | `src/window::opening_the_sample_draws_it_and_works_out_the_paths` | The logic is tested without a screen; no person has looked at the drawing. |
| ISOC-037 | The path of the tool is drawn with its depth from blue to amber. | prototype | partial | host-test | `src/toolpath::the_colours_go_from_blue_to_amber_with_the_depth_and_the_other_kinds_have_their_own` | The logic is tested without a screen; no person has looked at the drawing. |
| ISOC-038 | Every sentence of the window has a text in each of the eight languages. | implemented | done | host-test | `src/lang::every_sentence_has_a_text_in_each_of_the_eight_languages` | - |

## Not written yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ISOC-039 | The paths follow the exact curve, or arcs are fitted to them. | planned | pending | none | `planned:curve_paths::the_path_follows_the_curve_within_the_tolerance` | Exact to half a cell today. |
| ISOC-040 | A Voronoi strategy traces the paths. | planned | pending | none | `planned:voronoi::the_voronoi_paths_isolate_the_same_copper` | Not written. |
| ISOC-041 | A silkscreen layer is written. | planned | pending | none | `planned:silkscreen::the_silkscreen_layer_is_written` | No silkscreen layer in the job. |
| ISOC-042 | The checks `check`, `drc` and `preview` are in the window. | planned | pending | none | `planned:window_checks::the_window_runs_check_drc_and_preview` | They stay on the command line today. |
| ISOC-043 | A board is milled with the paths and measured against the design. | planned | blocked | none | `planned:milled_board::a_milled_board_matches_the_design` | Needs a machine, a board and a person. |
| ISOC-044 | The files written are read by the Bungard routing program and by a machine. | planned | blocked | none | `planned:other_readers::the_files_are_read_by_the_programs_of_the_machine` | Needs those programs and a machine. |
| ISOC-045 | The window is looked at on a screen and matches the picture the program takes of itself. | planned | blocked | none | `planned:window_screen::the_window_matches_the_reference_picture` | Needs a person at a screen. |
