# MSC 2 — Origin

**Owner:** Cameron Temple

This is the story behind MSC 2: why it started, what problem it grew out of,
and why MSC 2 is a separate product from MSC 1. For the short explanation of
what MSC 2 does, start with the [README](../../README.md). For the product
requirements and current direction, see the [product document](msc2-product.md).

## Why I built this

I really just wanted to play Minecraft with my people.

I had switched from Bedrock on Console to Java on my MacBook, but most of my
people were still playing Bedrock. So if I wanted everybody in the same world,
I needed cross-play.

That meant setting up a Paper (which is a type of Minecraft server for Java)
server with Geyser and Floodgate (which are plugins for Paper that allow for
Java <-> Bedrock crossplay).

At the time, I had basically never used a terminal before.

So now, just to play Minecraft, I had to figure out Java, server jars, plugins,
config files, ports, and how all of these pieces were supposed to fit together.
And even after I got it working, I still had to keep it working. Paper could
update before Geyser supported the new version, so something that worked
yesterday would suddenly stop, and I'd be back in the terminal replacing files
and restarting the server to find out whether I'd fixed it.

It was just a lot of stuff to deal with when all I was trying to do was play
Minecraft with my friends.

So I built something to handle it for me. First a rough script built in
python, then a real Mac app called **Minecraft Server Controller (MSC)** that I
taught myself Swift to write.

MSC 1 worked, and I used it for a long time. But two limitations eventually
became hard to ignore.

First, it was a Mac app. If you didn't have a Mac, you couldn't use it.

Second, the app *was* the server. The graphical program and the thing actually
running Minecraft were one and the same, so your server only ran while the app
was open, on the machine it was open on. The spare MacBook I used as a server
only had 8 GB of RAM, and I didn't want to burn a chunk of that running macOS
and a full GUI on a computer that mostly sat in a corner with the lid shut.

I wanted to run the server on whatever made sense — Linux, Windows, some cheap
little box with no monitor at all — and control it from somewhere else.

MSC 1 fundamentally wasn't built that way.

That's where **MSC 2** came from.

MSC 2 separates those two things: the engine runs in the background, while the
Tauri desktop app, desktop browser, and CLI are just ways to control it.

And somewhere along the way, this stopped being a nicer way to launch Paper.

Cross-play led to Geyser and Floodgate. Playing with people outside my house
led to port forwarding and Playit.gg. Playing with people on consoles led to
Xboxbroadcast. Then came backups, world management, modpacks, world
conversion, server health, crash handling, and all the other little problems
you run into when you host Minecraft yourself.

So now the goal is to fold that whole pile of problems into one application.

The people I imagine using this are pretty different.

Maybe you know Linux, you're perfectly comfortable in a shell, and you just
want a good headless Minecraft server manager. That's fine.

But maybe you just want to play with your niece, your siblings, or your
friends. One person is on a computer. Somebody else has an Xbox. You've never
opened a terminal and you have no desire to learn what a 'xyz' is tonight.

There's a funny bit of irony here. I built MSC because I was trying to get away
from the terminal. Now MSC 2 is being built to run headless and be controlled
through one lol.

MSC 2 should work for you too.

## Why I'm building this

There are already other ways to host Minecraft servers, and some of them are
really good. I'm not pretending I invented Minecraft server management.

MSC originally started partly because I wanted to learn how to build something
like this. But then I kept using it, kept running into new problems, and kept
adding ways to solve them.

## MSC 1

[MSC 1](https://github.com/ctemple9/minecraft-server-controller) is still its
own project.

It's a mature macOS app with around **97,000 lines of Swift**, and it already
does most of the things that eventually led to MSC 2. The problem is that it
was built around one Mac application, with the server management logic living
inside the GUI.

MSC 2 is built differently, so I'm not slowly converting MSC 1 into MSC 2.

They don't share configuration or live state, and MSC 2 doesn't reach into an
MSC 1 installation and start changing things. Moving a server between them is
something you explicitly choose to do through an import.

But MSC 1 is still really important to MSC 2 because it's the reference for
how a lot of this stuff is supposed to work.

Instead of rewriting a feature from memory and hoping I got all of the
behavior right, I can compare it against the application I've already been
using.

Two independent audits of MSC 1 agreed at the file level on **88.6% of its
246 source files** and identified roughly a third of the codebase as engine
logic worth carrying forward.

So MSC 2 isn't really me throwing MSC 1 away and starting from zero.

It's me taking what worked, separating it from the parts of the architecture
that eventually got in the way, and rebuilding it around what I actually want
MSC to be.

→ [MSC 1 on GitHub](https://github.com/ctemple9/minecraft-server-controller)
