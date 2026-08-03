# re2line

A speedrun analysis tool for Resident Evil 2 (1998). Only the Sourcenext 1.1 PC version is supported.

## Overview

re2line is a tool for recording your gameplay during speedruns and analyzing it afterward. It's intended to help
speedrunners answer questions like:

- What's the most optimal line through this room?
- What did I do differently on this run that made it faster/slower than the last run?
- Did I get hit because I messed up the line or because I got bad RNG?
- Does the game have any opportunities for RNG manip?
- Why won't Leon pick up the goddamn MO disk even though it's right in front of his stupid face?

To support this, it records the following information:

- Player position and movement
- Enemy position and movement
- AI information
- RNG rolls
- Inputs
- Other relevant game state

There are two main components:

- re2fr, aka the "Flight Recorder," is a
  [Classic Rebirth](https://classicrebirth.com/index.php/downloads/resident-evil-2-classic-rebirth/) mod which records
  your gameplay in real time.
- re2line is a GUI application for visualizing the recordings produced by re2fr.

Note that, because re2fr is loaded as a Classic Rebirth mod, it can't currently be used alongside the Speedrun Edition
mod, as Classic Rebirth only supports one mod at a time. This means it's mainly useful for recording practice sessions
rather than live runs. This limitation will likely be lifted in the future by providing alternative methods to inject
re2fr into the game process.

## Basic Usage

Start by dropping mod_re2fr.7z into the game folder. This will make the "Flight Recorder" mod available to select in
Rebirth's mod dropdown. Select that mod and start the game. This will create a file in the game folder called
re2fr_YYYY-MM-DD_HH-MM-SS.bin, where YYYY-MM-DD and HH-MM-SS are the date and time of the recording. Gameplay data will
be continuously written to this file as you play until you quit the game. Feel free to rename this file to something
more descriptive once you've closed the game.

Once you've created a recording, launch re2line.exe. Before you can open a recording, you'll first need to tell re2line
where the game folder is by going to File > Open game folder. re2line will remember this selection, so you'll only need
to do this once unless you're working with multiple game folders (for example, one for vanilla and one for cutscene
skip). Once you've done this, you'll be able to browse the list of rooms in the game and view collision and triggers
even without having a recording open. When you are ready to view a recording, use File > Open recording to select the
recording file you want to view. Note that it is possible to open a recording that's still in progress, but it won't
update in real time as you continue to play; you'll need to reopen the recording each time you want to refresh the data.

The app has a number of different views and tools to help you analyze your gameplay. These are described in detail
below.

## Room View

The room view occupies the majority of the window and shows an overhead view of the current room and the objects in it.
Specifically, it shows the following objects (note that most colors are defaults and can be changed in the
[Settings](#Settings) tab):

- **Floors**: The different floors of a room that a character or object can be on. For example, a room with a climbable
  staircase would have at least two floors, the lower one and the upper one. The game may also define additional floors
  for gameplay purposes. For example, in the room where you push the statues to get the red jewel, the spots you need
  to push the statues to are defined as separate floors. Floors are displayed as solid pink rectangles.
- **Collision**: Solid objects that characters can't pass through. Usually background objects. Some colliders only
  apply to the AI and not the player. Colliders may also be dynamically enabled and disabled by the game. Colliders
  are displayed as green, yellow, or black outlines. Green means the collider affects all characters, yellow means it
  only affects the AI, and black means that it's disabled.
- **AOTs (triggers)**: Triggers that the player can activate or interact with. These include doors, item pickups,
  message text, water, and more. Some triggers are activated by the player pressing the action button, while others are
  activated by the player entering the trigger area. AOTs are displayed as semi-transparent rectangles of varying colors
  depending on their type.
- **Objects**: Non-character 3D objects in the room; for example, pushable objects. Only present if a recording is open.
  Objects are displayed as white rectangles.
- **Characters**: Characters in the room. Only present if a recording is open. Characters appear as solid circles
  inside square outlines. The player and neutral characters are displayed in green, the player's ally (partner
  character) in light blue, and enemies in yellow. Characters are discussed in more detail in the
  [Characters](#Characters) section.

The room view additionally supports the following controls:

- Zoom in and out using the mouse wheel.
- Pan the view by middle-clicking and dragging the mouse.
- Hovering over an object with the mouse will display a tooltip with information about that object.
- Clicking on an object with the mouse will select it and show its details in the [Detail View](#Detail-View). The
  selected object is highlighted and renders on top of other objects.
- If a recording is open, you can play or pause the recording by using the space bar (currently requires the mouse to be over 
  the room view), or by clicking the play/pause icon next to the timeline beneath the room view.
- If a recording is open, you can use the left and right arrow keys to move the timeline forward and backward. When the
  recording is paused, the arrow keys will move the timeline by one frame at a time. When the recording is playing, the
  arrow keys will move the timeline by one second at a time.

Finally, when a recording is open, the recorded player inputs are displayed in the top-right corner of the room view.
The recorded inputs are forward, backward, left, right, action, run/cancel, and aim. The corresponding input icon will
light up if that button was pressed on the current frame. Only in-game inputs are recorded, not inputs in menus.

### Detail View

The detail view appears directly below the room view. At a minimum, the last line of the detail view will always show
the X and Z game coordinates of the point under the mouse cursor (the Y axis is the vertical axis and is not used by
most calculations). If a recording is open, the timeline will appear at the top of the detail view. From left to right,
the timeline consists of:

- The play/pause button.
- The timeline itself, a slider which you can drag to move through the recording.
- The current frame number.
- The current IGT time in the recording, formatted as minutes:seconds:hundredths.

If an object is selected, details about the object will appear between the timeline and the mouse coordinates. These
details include the object's type, name, position, and any other relevant properties or metadata.

### Characters

The display of characters in the room view is more complicated than other types of objects. To begin with, characters
are displayed as solid circles inside square outlines. This is to visualize how the game handles collisions. When a
character collides with a round edge, the game treats the character as a circle, but when colliding with a straight
edge, the game treats the character as a square. This is why, for example, it's possible to get stuck on the corner
when rounding the desk in the RPD lobby. The round part of the desk is made of a series of circle colliders, while the
final straight part is made of a single rectangle collider. As you pass the round part of the desk, the corner of your
square hitbox may intersect with the circle colliders, but this is fine because those are only looking at your circular
hitbox. But once you reach the straight part of the desk, if you're too close, the corner of your square hitbox will
hit the rectangle collider, stopping you dead in your tracks.

Each character also has an arrow pointing outwards from their center, which indicates the direction they're facing.
This is useful for determining the optimal line through a room. For the player character, you'll also see a small circle
displayed along the arrow. In order to interact with a trigger – such as a door, item pickup, switch, etc. – this
interaction point must be inside the trigger area. A black outline will display around the trigger area if the
interaction point is inside it. This can help to identify whether you missed a trigger, or how far away you can be while
still being able to interact with it.

By default, each character has a tooltip displayed above them which shows their ID number, name, AI state, and HP. The
AI state is a series of four hexadecimal numbers indicating the action the character is currently performing. There are
a huge number of possible AI states, which are not the same for every type of character, and not all of their meanings
are known. However, if you select a particular character, the detail view will display a brief description of the
current AI state if one is known.

When an enemy character is in a known AI state, the room view will display the enemy's AI zones. AI zones are regions
that will trigger an enemy action if the player enters them. Some zones may always trigger a particular action, while
others may roll RNG to determine whether to take an action or which action to take. You can hover the mouse over a zone
for a tooltip describing what the AI will do if the player enters the zone. Zones are displayed with a black outline
when the player is inside them. Most zones are circles or arcs with a solid fill color, but some are drawn as an
outline, which indicates that the AI is checking for the player to be *outside* of the zone instead of inside it. AI
zones are grouped into four categories represented by different colors:

- **Aggro** zones are shown in orange. Entering an aggro zone can cause an idle enemy to begin moving towards the
  player.
- **Attack** zones are shown in red. Entering an attack zone can cause the enemy to attack the player.
- **Hit** zones are shown in purple. Entering a hit zone will result in the player being hit by an ongoing attack.
- **Tactic** zones are shown in blue. Entering a tactic zone can cause the enemy to switch to a different non-attack
  behavior.

Some AI behavior is based on sound. Lickers are the obvious example, but other enemies may also use sound to trigger
actions, such as zombies deciding whether to lunge at you when you pass behind them. To help keep track of sound-based
AI behavior, an icon is displayed at the player's current location each time the player makes a sound. The icon will
stay at that location but fade over time, so you can see a record of all the most recent sounds the player has made.
There are five different sounds represented by the following icons:

- 🔫: A gunshot
- 👞: A walking footstep
- 👟: A running footstep
- 🔪: Using the knife
- 🎯: Aiming your weapon

When the player is aiming or firing their weapon, the room view will also display the weapon damage ranges. Weapons in
RE2 do more damage at close range, but the falloff is not continuous; there are three distinct ranges, each with an
associated damage value. The three ranges are represented by dark blue rectangular outlines originating from the player
character and extending outwards in the direction the weapon is aimed. When two ranges overlap, the game will use the
further (i.e., lesser damage) one.

Lastly, when a character is selected, the detail view will include a Display section with options controlling how the
character is displayed in the room view. The options are:

- **Show character**: If enabled (the default), the character will appear in the room view. Otherwise, the character
  will be hidden.
- **Show tooltip**: If enabled (the default), show the informational tooltip above the character at all times.
- **Show AI**: If enabled (the default), show the character's AI zones. Otherwise, the AI zones will be hidden.
- **Show path**: If enabled, the character's path through the room will be drawn on the ground behind them as they move.
  This can be used to visualize the player's or an enemy's movement. The path will change color to indicate the
  character's speed at that point, with green being faster and red being slower. This is off by default.

## Browser

The browser panel appears on the left side of the window and contains a few different tabs allowing you to browse
through various game objects and application settings.

### Game

The Game tab contains a list of all the rooms in the game. Rooms are organized into separate Leon and Claire sections,
as these characters have their own copies of each room. Open the section for the desired character and click on a room
to open that room in the room view. Each room is identified by a four-digit hexadecimal ID. The first digit is the
stage, the second and third digits are the room number within the stage, and the fourth digit indicates the character,
`0` for Leon and `1` for Claire.

### Room

The Room tab lists all the objects in the current room. At the top of the tab, if a recording is open, the following
stats about the current pass through the room will be displayed:

- **Frames**: How many frames the player spent in the room.
- **Time**: How much IGT time the player spent in the room.
- **RNG rolls**: How many times RNG was rolled in the room.
- **RNG index**: The index in the list of possible RNG values that RNG was at when the player entered the room.
  See the [RNG](#RNG) section for more information.

Below this (or at the top of the tab if no recording is open) is the "Print scripts" button, which can be used to print
the room scripts to the console.

Beneath this header section is the list of objects in the room. Objects are grouped into seven categories: Floor,
Collision, Door, Item, AOT, Objects, and Characters. See the [Room View](#Room-View) section for more information about
these categories. Selecting an object from the list will select it in the room view.

### Recording

The Recording tab, which appears only if a recording is open, serves as a table of contents. The tab shows a list of
"runs" in the recording. A run is considered to begin when you start a new game or load a save and end when you return
to the main menu. Each run is labeled with a sequential number and the name of the scenario you were playing. Clicking
on a run opens it to reveal the list of all rooms that were visited during that run, in order. Each room in the list
shows the room ID, the IGT time at the time the room was entered, and the frame count at the time the room was entered.
Clicking on a room in the list will open that room in the room view and seek the timeline to that time in the recording.

### RNG

The RNG tab appears only if a recording is open and shows a list of RNG rolls that have occurred in the current room
up to the current time in the recording. The game uses RNG for a huge number of actions and effects, and the effects of
most RNG rolls have not been determined yet. Many have, however, and for those, this tab will show information about
what the game was using RNG to determine and what the outcome was.

The tab begins with a few options to control which types of rolls to display:

- **Show character rolls**: Show RNG rolls associated with characters. A zombie deciding its starting health or a licker
  deciding whether to jump would be examples of character rolls.
- **Show known non-character rolls**: Show RNG rolls that are not associated with characters but whose effects are
  known. The chance for the handgun to crit on hard mode would be an example of a known non-character roll.
- **Show unknown rolls**: Show RNG rolls whose effects are not known.
- **Characters**: The Characters section allows you to filter which characters' rolls are shown when "Show character
  rolls" is enabled. You can either manually select individual characters to include or use the "Select all" and
  "Select none" buttons to toggle all characters.

Below the settings is the list of RNG rolls. The list is displayed in descending order, meaning more recent rolls are
at the top. Rolls are grouped by the frame that they occurred on. Each frame shows the IGT time on the frame, the frame
count on that frame, and the number of rolls that occurred on that frame. The rolls listed within each frame are text
descriptions of what the roll was checking for and what the outcome was, if the purpose of the roll was known. For
example, a roll description might read "#4 Zombie (random) rolled for health: 40 (index 8)". This means that the
character with index 4, which was a zombie with a random appearance, rolled to determine its starting health. The result
was 40 HP, which is the value at index 8 in the list of possible starting health values. For unknown rolls, the
description will look like "00452C57 rolled on 1670". This means that the RNG call occurred at memory address 00452C57
and the current RNG value at the time was hexadecimal 1670. Another recurring description you'll see is "Partial roll
in a larger series". Sometimes the game will roll RNG two or three times and combine the results into a single value.
When this happens, the initial roll(s) will be labeled as "Partial roll in a larger series" and only the final roll
will get the full description.

You can right-click on a roll to open a pop-up with some additional information:

- **RNG index**: The index in the list of possible RNG values that RNG was at when the roll occurred. While there's
  not actually a hard-coded list of possible RNG values in the game, the RNG function is deterministic and ultimately
  repeats itself. This means that we can think of it as a list that we just loop through over and over; specifically,
  a sequence of 24,312 numbers. The start of the list (position 0) is defined to be the position of the RNG function's
  initial seed.

The remaining items are only displayed for known rolls:

- **Next unique value**: This shows, when we look forward in the RNG sequence, how many more rolls it would take for us
  to have gotten a different outcome for this event than we did. This is mostly useful for events with only two possible
  outcomes. For three or more possible outcomes, it may be more helpful to use the [RNG explorer](#Explore-RNG).
- **Previous unique value**: Same as above, but in the other direction – how much *earlier* in the RNG sequence would
  we have to have gotten here to get a different outcome?
- **Explore**: A link to open this roll in the [RNG explorer](#Explore-RNG).

### Settings

The Settings tab contains various settings for the application itself. The following settings are available:

- **Focus for current selection**: When enabled, fade objects that are not the same floor as the selected object. This
  makes it more clear which objects can currently affect the selected object.
- **Alternate collision colors**: When enabled, disabled colliders will be drawn in black, and AI-only colliders will be
  drawn in the Enemy color. Otherwise, all colliders will use the selected Collider color (default green).
- **Show character tooltips by default**: When enabled, automatic tooltips will be enabled for all characters by default.
  If you turn this off, you can still enable tooltips for specific characters from the character detail view.
- **Show sounds**: When enabled, show a trail of sound icons at points where the player makes a sound.
- **Show all objects**: When enabled, show all 3D objects in the room. Otherwise, only 3D objects that the player can 
  interact with are shown.

The remainder of the tab contains settings to control the appearance of the various types of objects shown in the room
view. Unchecking the "Show" option for an object type will hide all objects of that type. You can also change the color
used for that object type.

## Tools

The tools menu contains miscellaneous tools for analyzing gameplay.

### Compare runs

This tool allows you to compare what happened in a single room across multiple runs. Selecting this option will open the
Compare Runs window. The process assumes that the room you want to compare is the one that you're currently looking at.
However, since we visit some rooms multiple times in the run, you can optionally apply filters to narrow down which
visit to the room you want to compare. The possible filters are:

- **Entrance filter**: Include only visits where the player entered the room from the selected other room.
- **Exit filter**: Include only visits where the player exited the room to the selected other room.
- **Required triggers**: Select one or more triggers which the player must have interacted with for the visit to be
  included.

Click "Confirm and select recordings" to be prompted to select the recording files you want to use for the comparison.
Every matching visit to the room across all runs in all files will be included in the comparison.

After selecting your recordings, the Recording tab will change to the Comparison tab. The Comparison tab shows a list of
all the runs included in the comparison. At the top of the tab are the following statistics:

- **Runs**: How many runs are included in the comparison.
- **Fastest**: The IGT time and frame count of the fastest run.
- **Slowest**: The IGT time and frame count of the slowest run.
- **Average**: The average IGT time and frame count of all runs.

There are also a couple settings that can be tweaked:

- **Include exclusions in statistics**: If enabled, runs that you've excluded below will still be included in the
  fastest, slowest, and average times.
- **Show paths**: If enabled, the path the player took through the room will be shown in the room view for each included
  run. The fastest run is shown in gold and the selected run (if it isn't also the fastest) is shown in blue. The rest
  of the runs are color-coded red to green based on how fast they are compared to the gold run, with red being slower 
  and green being faster.

Above the list of runs are "Select all" and "Select none" buttons that can be used to toggle all runs in the comparison.
Each run also has an "Include" toggle to control whether it's included in the comparison. The run is labeled with the
filename of the recording it came from and the frame count at the time the player entered the room. The IGT time and
number of frames spent in the room on that run are also displayed.

Other than the paths shown by the "Show paths" setting, the rest of the room appears as it was in the run that you have
selected in the Comparison tab. Likewise, the RNG tab also shows information only for the selected run. When you're done
with the comparison, use File > Close comparison to exit comparison mode.

### Explore RNG

This tool allows you to analyze the frequency of RNG events, as well as search for "runs" in the RNG sequence where
some desired outcome occurs multiple times in a row. The "Explore RNG" window is divided into a few different sections.
The top section has the following options:

- **Roll type**: The type of RNG roll to analyze. Only known rolls are available to select.
- **RNG index**: The index in the list of possible RNG values where you want to do the analysis.