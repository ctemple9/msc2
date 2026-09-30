scoreboard players set #msc_clock msc_map_players 0
scoreboard players add #msc_sequence msc_map_players 1
scoreboard players get #msc_sequence msc_map_players
scoreboard players get #msc_tick msc_map_players
execute in minecraft:overworld run scoreboard players get #msc_dimension_overworld msc_map_players
execute in minecraft:overworld as @a run function msc_map_players:sample/player
execute in minecraft:the_nether run scoreboard players get #msc_dimension_nether msc_map_players
execute in minecraft:the_nether as @a run function msc_map_players:sample/player
execute in minecraft:the_end run scoreboard players get #msc_dimension_end msc_map_players
execute in minecraft:the_end as @a run function msc_map_players:sample/player
scoreboard players get #msc_sequence msc_map_players
