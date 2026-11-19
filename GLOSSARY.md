# poe-overlay

A desktop companion for levelling through the Path of Exile 1 campaign: it tracks the run from the game log and shows the guide, timer, gems, passive tree and lab as overlays on top of the game.

## Game state

**Client Log**:
The `Client.txt` file the game appends to; the only automatic source of game state.
_Avoid_: log file, chat log

**Log Event**:
One meaningful fact parsed from a Client Log line, such as entering an area, levelling up or dying.
_Avoid_: message, log line (a line is raw text; an event is what it means)

**Area**:
A single game location identified by an **Area ID** (e.g. `1_1_2`) and an area level.
_Avoid_: map (reserved for endgame maps), level (ambiguous with character level)

**Zone**:
_Avoid_: use **Area**

**Town**:
The safe hub Area of an act, where its Split is taken.

**Hideout**:
A player-owned Area outside the campaign; time spent here can be auto-paused.

**Character**:
A named in-game character with a class and level, as reported by the Client Log.
_Avoid_: toon, char

**Profile**:
Everything the app keeps for one Character: imported Build, Run Category, Guide progress and its current Run.
_Avoid_: account, save

**Active Character**:
The Character believed to be played: the last one to level up, or the one the player picked. Only its Run counts.

**Pending Run**:
A Run started by a fresh Twilight Strand before any Character has claimed it by reaching level 2.

## Runs and timing

**Run**:
One attempt at the campaign by one Character, from its first Twilight Strand to reaching the Epilogue.
_Avoid_: attempt, session

**Epilogue**:
Karui Shores, the Town reached after defeating Kitava; reaching it finishes a Run.

**Real Time**:
Elapsed Run time excluding Pauses.

**Load-Removed Time**:
Real Time minus loading screens that occurred while the Run was not paused.
_Avoid_: game time, IGT

**Split**:
The Real and Load-Removed Time at which a Run completed an act: first reaching the next act's Town, or the Epilogue for act 10.
_Avoid_: lap, checkpoint

**Run Category**:
A label grouping comparable Runs, such as "SSF HC" or "Trade SC"; Personal Bests are kept per category.
_Avoid_: mode, league (a league is the game's season, a category is ours)

**Personal Best**:
The fastest completed Run in a Run Category, against which live Splits are compared.
_Avoid_: PB record, best run

**Best Possible Time**:
The sum of the fastest individual act segments across all Runs in a Run Category.
_Avoid_: sum of best, gold splits

**Pause**:
A stretch of a Run that does not count toward elapsed time; either **Manual** (by hotkey) or **Automatic** (at login or character select, or in a Hideout).

## Guide

**Guide**:
The full ordered sequence of Steps for the campaign.
_Avoid_: route (reserved for the upstream source data), walkthrough

**Route**:
The upstream exile-leveling source data a Guide is converted from.

**Step**:
One instruction in the Guide, such as entering an Area, killing a boss or taking a Quest Reward.
_Avoid_: line, task

**Current Step**:
The Step the player is believed to be on; advances automatically only forwards.

**Bandit Choice**:
Which of the three Act 2 bandits the player helps, or kills all; changes the Steps shown.

**Direction Hint**:
A compass bearing attached to a Step pointing toward the exit or objective in an Area.
_Avoid_: layout (reserved for full Area layout images, not yet supported)

## Build and gems

**Build**:
A Path of Building export imported into a Profile: class, ascendancy, Bandit Choice, Skill Groups and Tree Stages.
_Avoid_: PoB, loadout

**Skill Group**:
A set of linked gems from the Build intended for a level range.
_Avoid_: link, setup

**Quest Reward**:
A gem the Character may choose for free on completing a quest, depending on class.

**Vendor Reward**:
A gem that becomes purchasable from a specific NPC after a quest, depending on class.

**Vendor Regex**:
A search string for the in-game vendor search box that highlights the gems to buy at the Current Step.

## Passive tree

**Tree Stage**:
One of possibly several passive trees in a Build, each targeting a character level.
_Avoid_: spec (Path of Building's word), tree version

**Next Points**:
The ordered list of passive nodes the Character should allocate next toward the current Tree Stage.

**Calibration**:
Aligning the tree Overlay onto the in-game passive tree: a middle-click on the class start with the tree fully zoomed out (scale follows from the game window height).

## Labyrinth

**Lab Layout**:
The day's labyrinth room map, imported from a link the player copies from poelab.

**Trial**:
One of the six campaign Labyrinth trial rooms; completing all is required to enter the Labyrinth.

## App

**Overlay**:
A transparent, click-through window drawn over the game showing one feature.
_Avoid_: HUD, widget, popup

**Interact Mode**:
A toggled state in which Overlays accept the mouse so they can be moved, resized or clicked.
_Avoid_: edit mode, unlock

**Replay**:
Feeding a recorded Client Log through the app as if it were being written live.
