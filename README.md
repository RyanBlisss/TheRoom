# The Room

**The Room** is a 2D atmospheric horror game built around sound, tension, shifting spaces, and psychological pressure.  
The core experience is not just what the player sees, but what they hear.

The player enters the Oceancrest Hotel expecting a peaceful vacation. Instead, the hotel becomes a living maze of locked rooms, distorted reality, hostile soundscapes, and a hunting entity that grows more dangerous over time.

---

## Table of Contents

- [Game Concept](#game-concept)
- [Core Design Pillars](#core-design-pillars)
- [Gameplay Loop](#gameplay-loop)
- [Main Objective](#main-objective)
- [Room System](#room-system)
- [Sanity System](#sanity-system)
- [Monster System](#monster-system)
- [Items](#items)
- [Locations](#locations)
- [Characters](#characters)
- [Story](#story)
- [Development Notes](#development-notes)

---

## Game Concept

**Genre:** Audio-based atmospheric horror  
**Perspective:** 2D  
**Setting:** A haunted hotel with shifting rooms  
**Tone:** Psychological horror, isolation, dread, distorted reality

Audio is more important than visuals in this game. The player should rely heavily on sound cues, environmental noise, music, whispers, distortion, and silence to understand danger, direction, and story clues.

AI-generated background songs and atmospheric layers are used to make the hotel feel unstable, haunted, and unpredictable.

---

## Core Design Pillars

### Audio-First Horror

The game should make sound feel like a core mechanic, not just decoration.

Examples:

- distant footsteps
- ventilation movement
- whispers from walls or vents
- distorted music
- room-specific ambience
- silence before danger
- directional monster cues
- sanity-based audio hallucinations

### Shifting Reality

Rooms change after doors are unlocked or after the monster catches the player. The hotel should feel unreliable and alive.

The player should never feel fully safe or fully familiar with the layout.

### Escalating Fear

The more rooms the player unlocks, the scarier the hotel becomes.

Progression should introduce:

- stronger visual horror
- more aggressive audio
- altered room layouts
- new hallucinations
- returning objects in different places
- increased monster intelligence

### Psychological Pressure

The sanity system should make the player feel that every mistake matters.

Permanent sanity loss creates long-term pressure, while temporary sanity loss creates short-term urgency.

---

## Gameplay Loop

1. Explore the hotel.
2. Search for keys and useful items.
3. Unlock a new room.
4. The hotel layout changes.
5. Sanity permanently decreases.
6. The monster becomes more dangerous.
7. Audio and visual horror intensify.
8. Repeat until every room is unlocked.
9. Enter the final room: **The Room**.
10. Survive the final encounter and escape.

---

## Main Objective

The player must unlock all rooms in the hotel.

The exit cannot be accessed immediately because the final key is hidden in the last room. This final room is called **The Room**.

To escape, the player must:

- unlock every room
- survive the changing hotel layout
- manage sanity
- avoid or counter the monster
- find the final exit key
- survive the final face-to-face encounter

---

## Room System

The hotel contains multiple locked rooms. Each time the player unlocks another room, the hotel changes.

### Room Changes

When rooms change:

- room positions may shift
- key locations may move
- previously explored rooms may become different
- visual atmosphere changes
- audio atmosphere changes
- horror elements increase
- the monster state may become more threatening

The player never knows exactly where the next key is unless they explore.

### Permanent Sanity Cost

Each major room change permanently reduces sanity by **20%**.

This permanent sanity damage cannot be healed with sanity pills.

---

## Sanity System

Sanity is split into two types of loss:

### Temporary Sanity Loss

Temporary sanity decreases over time or because of fear events.

This can be healed using sanity pills.

### Permanent Sanity Loss

Permanent sanity decreases when the hotel layout changes.

Each room change permanently lowers sanity by **20%**.

Sanity pills cannot restore permanent sanity loss.

### Sanity Pills

Sanity pills restore only temporary sanity loss.

They do not reverse damage caused by room changes.

---

## Monster System

The monster is not present from the start. It appears after the player uses the first key to unlock the next door.

### Monster Behavior

Every minute, the monster teleports to another room that does not currently contain the player. In story terms, it moves through the ventilation system.

The monster becomes more dangerous the longer the player survives.

### Escalation

Over time, the monster:

- moves faster
- tracks the player more intelligently
- gets closer to the player more often
- visits locations the player frequently uses
- becomes more aggressive as the game progresses

### Catch System

Maximum catches: **8**

When the monster catches the player:

- rooms change
- visuals change
- audio changes
- the hotel becomes more hostile

---

## Items

### Keys

Used to unlock rooms and progress through the hotel.

### Wind-Up Toy

A distraction item that may be used to lure or misdirect the monster.

### Sanity Pills

Restore temporary sanity loss only.

They cannot heal permanent sanity damage.

### CD

A collectible or usable audio item.

Potential uses:

- story clue
- puzzle solution
- music trigger
- frequency mechanic
- distraction

### CD Player

Used to play CDs.

Potential gameplay use:

- trigger story memories
- play calming audio
- activate sound-based puzzles
- interact with the monster weakness

---

## Locations

## First Floor

- Main room / player starting room
- Bathroom
- Bedrooms
- Hall
- Kitchen
- Dining room

## Second Floor

- Bedrooms
- Bathroom
- Hallways
- Hole in the center
- Staircase

The second floor does not include:

- kitchen
- dining room
- main room

---

## Characters

## Blarg Thompson

**Role:** Player character

| Attribute | Detail |
|---|---|
| Full Name | Blarg Thompson |
| Gender | Male |
| Age | 32 |
| Height | 5'11" / 180 cm |
| Weight | 165 lbs / 75 kg |
| Eye Color | Hazel |
| Hair Color | Dark Brown |
| Ethnicity | Caucasian |
| Skin Color | Light |
| Marital Status | Single |
| Occupation | Software Developer |

### Background

Blarg grew up in a small town in the Midwest. He later moved to a large city for college, where he developed an interest in technology and urban exploration.

His fascination with urban legends began after an encounter with a mysterious local legend during his college years.

### Personality

Blarg is curious, logical, and slightly skeptical of the supernatural. He approaches problems methodically and relies on reason, but he is deeply fascinated by mysteries.

### Strengths

- analytical thinking
- technical knowledge
- puzzle solving
- resourcefulness

### Weaknesses

- overly logical
- sometimes dismissive of emotions
- mild claustrophobia

---

## Isaac Remington / The Entity

**Role:** Main antagonist

| Attribute | Detail |
|---|---|
| Full Name | Isaac Remington |
| Gender | Male |
| Age | 45 |
| Height | 7'0" / 213 cm |
| Weight | Variable |
| Eye Color | Black with a faint red glow |
| Hair Color | None |
| Ethnicity | American, but features are distorted |
| Skin Color | Pale, almost translucent, with visible veins |
| Marital Status | Single |
| Former Occupation | Neuroscientist and AI researcher |

### Background

Dr. Isaac Remington was a brilliant neuroscientist who developed an AI capable of interfacing directly with the human brain.

After being diagnosed with aggressive brain cancer, he replaced parts of his brain and organs with AI-enhanced prosthetics. The transformation had catastrophic side effects, turning him into a malevolent entity.

He now haunts the hotel, manipulates reality, and preys on guests' sanity.

### Personality

Isaac is malicious, cunning, and patient. He enjoys psychological torment and prefers to drive victims close to madness before attacking.

### Strengths

- manipulates the environment
- induces hallucinations
- teleports between rooms
- grows stronger as sanity decreases

### Weakness

Isaac is weak to light and a specific sound frequency:

```text
6798 Hz
```

---

## Evelyn Parker

**Role:** Ghostly hotel staff / helper character

| Attribute | Detail |
|---|---|
| Full Name | Evelyn Parker |
| Gender | Female |
| Age | Appears late 20s |
| Height | 5'6" / 168 cm |
| Weight | 130 lbs / 59 kg |
| Eye Color | Blue |
| Hair Color | Blonde |
| Ethnicity | Caucasian |
| Skin Color | Pale with a ghostly glow |
| Marital Status | Single |
| Former Occupation | Hotel Manager |

### Background

Evelyn was the kind and diligent manager of the hotel. She disappeared decades ago and now remains as a ghostly presence.

She helps Blarg navigate the hotel, although her memories are fragmented.

### Personality

Compassionate, resourceful, melancholic, and determined to help Blarg uncover the truth.

### Strengths

- knows the hotel's history
- understands parts of the layout
- can appear and disappear

### Weaknesses

- limited physical interaction
- fragmented and unreliable memories

---

## Michael "Mike" Williams

**Role:** Ghostly hotel guest / music-based support character

| Attribute | Detail |
|---|---|
| Full Name | Michael "Mike" Williams |
| Gender | Male |
| Age | 34 |
| Height | 6'0" / 183 cm |
| Weight | 180 lbs / 82 kg |
| Eye Color | Green |
| Hair Color | Black |
| Ethnicity | African American |
| Skin Color | Dark |
| Marital Status | Married |
| Occupation | Musician |

### Background

Mike was a traveling musician who stayed at the hotel during a tour. He is a friendly spirit haunted by his untimely death.

### Personality

Charismatic, humorous, optimistic, and emotionally supportive.

### Strengths

- creates calming music
- temporarily slows sanity loss
- provides emotional contrast to the horror

### Weakness

Mike is bound to the room where he died and cannot leave it.

---

## Linda Garcia

**Role:** Ghostly hotel guest / guidance character

| Attribute | Detail |
|---|---|
| Full Name | Linda Garcia |
| Gender | Female |
| Age | 28 |
| Height | 5'4" / 162 cm |
| Weight | 125 lbs / 57 kg |
| Eye Color | Brown |
| Hair Color | Dark Brown |
| Ethnicity | Hispanic |
| Skin Color | Medium |
| Marital Status | Engaged |
| Occupation | Teacher |

### Background

Linda was a school teacher staying at the hotel for a conference. She remains as a gentle spirit who tries to guide Blarg.

### Personality

Nurturing, intelligent, calm, and helpful.

### Strengths

- gives hints
- helps solve puzzles
- provides guidance

### Weakness

Her presence can attract The Entity, so she cannot appear too often.

---

# Story

## Prologue: Ocean Breeze, Fading Light

You are Blarg Thompson, arriving at the Oceancrest Hotel for what was supposed to be the perfect vacation.

The Florida sun is bright, the hotel looks luxurious, and the ocean view promises rest. At first, everything feels peaceful.

That peace does not last.

---

## Scene I: The Perfect Room

Blarg enters his hotel room.

Sunlight fills the suite. A queen bed, clean white sheets, a leather couch, a wall-mounted television, and a balcony view of turquoise waves all suggest comfort and safety.

For a moment, the hotel feels normal.

---

## Scene II: Beachside Reverie

Blarg heads to the beach.

There is music, warm sand, food from the tiki bar, and people enjoying the evening. But after a short time, he realizes he forgot sunscreen and heads back to his room.

This small decision leads him away from the normal world.

---

## Scene III: Unnerving Silence

Blarg returns to the lobby.

The music is gone. The tropical air feels muted. The front desk is empty. The hotel no longer feels inhabited.

He calls out, but only his own voice answers.

Something has changed.

---

## Scene IV: Lights Out

Blarg reaches his room.

The lights in the hallway and inside the room suddenly go out. The darkness feels heavy and unnatural.

A metallic sound echoes behind him.

There is no visible source.

---

## Scene V: First Ominous Clue

Inside the room, Blarg finds a folded note on hotel stationery:

```text
Dear Guest,

We hope you're enjoying your stay. For your safety, please remain in your room until further notice.

— Management
```

The note feels official, but wrong. There is no explanation, no signature, and no sense of comfort.

---

## Scene VI: The Whispering Vent

The vents begin to whisper.

Blarg hears fragments:

```text
"...don't trust the lights..."
"...he sees you..."
```

The hotel is no longer just empty. It is aware.

---

## Scene VII: Turning Point

The vacation is over.

The hotel has become a cage, and Blarg must survive using logic, caution, and anything he can find.

His first goals are clear:

- investigate the lobby
- follow the whispers carefully
- secure a light source
- find out who left the note

The Oceancrest Hotel is not what it seems.

The real journey begins in the dark.

---

# Development Notes

## Priority Features

- 2D exploration system
- locked-room progression
- dynamic room layout changes
- sanity system
- permanent and temporary sanity damage
- monster teleportation
- monster AI escalation
- audio-first environmental design
- key/item system
- final room encounter
- story notes and ghost encounters

## Audio Design Requirements

Audio should communicate:

- danger
- monster proximity
- room identity
- sanity level
- hallucinations
- hidden clues
- ghost presence
- environmental changes

The game should remain scary even with simple visuals.

## Possible Puzzle Mechanics

- finding keys after room shifts
- using the CD player to trigger memories
- playing 6798 Hz to weaken The Entity
- using the wind-up toy as a sound distraction
- following whispers to find clues
- avoiding misleading audio hallucinations
- using light sources to create temporary safety

## Possible Win Condition

The player wins by:

1. unlocking every room
2. entering **The Room**
3. surviving the final encounter with Isaac Remington
4. obtaining the exit key
5. escaping the Oceancrest Hotel

## Possible Lose Conditions

The player loses if:

- sanity reaches zero
- the monster catches the player too many times
- the player fails the final encounter
- the hotel fully consumes Blarg's reality

---

# Project Status

Current status: **Concept / early design document**

This README defines the core idea, mechanics, characters, and story foundation for the game.
