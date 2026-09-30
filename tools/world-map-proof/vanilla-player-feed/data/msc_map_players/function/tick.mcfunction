scoreboard players add #msc_clock msc_map_players 1
scoreboard players add #msc_tick msc_map_players 1
execute if score #msc_clock msc_map_players matches 20.. run function msc_map_players:sample
