# Big Late Spring Update

(Yes, it's a bit late, it's in the name)

Dan here with an update. It's a chonker, so here's a short overview before you delv… dive in:

1. Future of Coding appearance
2. The Big Plan
3. Funding
4. Team Changes
5. New Landing
6. Bonus Photos
7. Big Rewrite
8. Device Validation
9. Gaining Weight
10. Mascot

![A very good chonker](https://late-mate.com/email_resources/updates/004/chonker.jpg) 

([credit](https://commons.wikimedia.org/wiki/File:Cat_on_its_back.jpg))

Things are moving!

## Future of Coding appearance

I made a demo at my favourite London meetup, [Future of Coding London](https://lu.ma/foclondon):

[![YouTube preview of the demo recording](https://late-mate.com/email_resources/updates/004/focdemo.jpg)](https://www.youtube.com/watch?v=WZeCQwtMrCg)

([YouTube link](https://www.youtube.com/watch?v=WZeCQwtMrCg))

Few communities channel the creative, demo-forward London scene energy better than the FoC meetup.
Say hi if you come to the next one!

## The Big Plan

I talked a bit about the Big Goals in the [new landing page video](https://www.youtube.com/watch?v=36qOK8cBEDo)
(more on this later), but here I want to tell you all a bit more about the Big Plan.

To start a startup, one has to be a bit delusional. I'm just delusional enough to start, but not enough to
think that launching an end-user device will force every company on the planet to make and keep their interfaces snappy.

The only way to keep computers snappy is to sneak into release testing pipelines and to stay there.

In other words, to change personal computing forever, I need to make latency testing continuous and automated.
I need to build a CI SaaS. This is the endgame and a very nice place to be as a business! But to get there 
is to thread a very fine needle. Here's how I'll thread it:

Step 1: Late Mate campaign and general availability of the input-to-photon device

Step 2: Late Mate v2, measuring the delay between a simulated input and a video output frame

Step 3: Late Mate CI, using v2 hardware

Every step builds on top of the previous one, and every step is *hard*. But hey, remember what I said about being
a bit delusional?

## Funding

One thing I learned really fast starting this up is how everything startup is one big chicken-and-egg omelette! 
Improving the product requires focused work, focused work requires paying the rent, paying the rent requires sales, 
sales require having a product and marketing it, marketing is easier when the product is already great, GOTO10.

But then I realised I should just reach out and invite people to pitch in. Late Mate is my moonshot, a bet on
a few rocks starting an avalanche and flipping the industry into a new, better state. I know that some people 
love and look for endeavours of this sort. Some of you might be the people.

If you are, let's talk. Reply to this email or ping me at [dan@dgroshev.com](mailto:dan@dgroshev.com),
and let's change every computer out there, together.

## Team Changes

Since the last update, we had a reshuffle. I (Dan) switched to working on Late Mate full-time several months ago, 
and Niki decided to spend more time on his [other projects](https://tonsky.me/projects/). The number of projects Niki
is juggling is truly inspiring. Right now, I'm using his Fira Code! I'm deeply grateful for Niki's Late Mate 
contributions, and looking forward to our future collaborations. Love you, Niki ❤️

## New Landing

Late Mate got [a new landing](https://late-mate.com/):

[![New Late Mate landing](https://late-mate.com/email_resources/updates/004/landing.png)](https://late-mate.com) 

The latency demo on top of the page was a shower idea. Showing is better than telling, and I can replicate my 
*favourite* typing experiences with a bit of JS. It is a reminder of how annoying computer inputs can be,
and I hope you *enjoyed* it.

The latency demo was quick and fun to make, unlike the video at the bottom of the page.

It took four rewrites and reshoots to get to a satisfactory script and delivery. With the first three attempts, 
I went for natural delivery, but my test audience complained that it's boring and unnatural. Then I switched to 
the deep-fried, jump cut-rich, clippy style native to YouTube, and the test audience complimented it as natural 
and engaging. That's when I learned how strongly media sets expectations, and that I have to follow along.

(This is why this update is so long, newsletters are expected to be long, right?)

With the script nailed, I made *every* beginner mistake possible:

* I used natural light, which was changing while I was recording many, many takes of every beat. 
There is only so much I can do with colour correction, so white balance shifts across cuts.
* I ignored the level indicator, so the audio clips. I threw away half the takes because they clipped too hard.
* Of course, I didn't make pauses long enough to make cutting simple!

But the result is something I can live with, and I think the landing page works. Don't let me know if you disagree!

## Bonus Photos

To make that little looped video on the landing, I rented a macro lens. Here is a behind-the-scenes shot:

![Shooting a Late Mate demo video](https://late-mate.com/email_resources/updates/004/bts.jpg) 

I couldn't miss the opportunity to play with the most expensive lens I've ever held,
so here are some beauty shots of the current prototype:

![A macro photo of a Late Mate PCB](https://late-mate.com/email_resources/updates/004/beauty1.jpg)

![A macro photo of the photodiode inside the device](https://late-mate.com/email_resources/updates/004/beauty2.jpg)

![A photo of Late Mate with Doom on the background](https://late-mate.com/email_resources/updates/004/beauty3.jpg) 

## Big Rewrite

I should apologise. It's pretty far down the email, and it's only now that I will tell you that the version 
in the Future of Coding demo was mostly thrown away.

I got in touch with someone whose work I deeply respect and found that they were interested in a Late Mate prototype. 
I prepared and sent one out, but felt bad about sending them an unstable device with a clunky CLI.
As the parcel was on its way to *\[REDACTED\]*, I crunched and finished the ongoing rewrite of the entire stack, 
from the firmware to the CLI.

The CLI is pretty nice now:

![A screenshot of a few Late Mate CLI commands](https://late-mate.com/email_resources/updates/004/cli.png)

The host-side driver is now extracted into a separate crate and can be used independently of the CLI for automation. 
It will be opensourced and available This Summer™.

There is also a new TOML-based format for testing scenarios that I find pleasing:

![A screenshot of a Late Mate testing scenario](https://late-mate.com/email_resources/updates/004/scenario.png)

It was surprisingly tricky to make Late Mate easy to update but impossible to brick, even if I push out 
a buggy firmware version.
This is a topic for another update or a blog post, but for now I will say that 
[I really like the RP2040](https://dgroshev.com/blog/rp2040/)!
Its design helped a lot.

## Device Validation

Since the very first prototype, I was asking myself if I can trust the numbers I'm getting out of it.
It's a chicken-and-egg problem, again: how do you validate a device measuring a time period too short 
to measure with something else?

A solution I came up with is to have a separate device reacting to USB HID events with a negligible delay.
I hooked an LED to Raspberry Pi GPIO and wrote a small binary that listens to raw kernel USB events
and toggles the LED on and off.
I then pointed the LED at the Late Mate light sensor and [ran a normal test](https://x.com/dangroshev/status/1784694529141973351).
As expected, Late Mate measured a delay of about 1ms, which is the practical floor for a pre-3.0 USB HID device.

I can now trust the device to not add its own latency.

## Gaining Weight

The current Late Mate prototype is mostly plastic. Plastic is very light, and the device will shrink a lot 
in the next version, making it even lighter. Light is not ideal for this, as the device needs to hang straight.

I'm now experimenting to find how much weight I should add and how to distribute it. If you ever do this, 
get some tungsten putty from a fishing shop. It's non-toxic and about as heavy as steel:

![A photo of a piece of tungsten putty being weighed](https://late-mate.com/email_resources/updates/004/putty.jpg)

I really like it for playing with weight distribution. Late Mate with extra weight feels awesome, 
and I'm looking to add some steel to the next prototype.

## Mascot

[Snapping shrimp](https://en.wikipedia.org/wiki/Alpheidae):

- ridiculously cool but annoying (to sonar operators)
- punch above their weight
- please reread their name

I really have no choice. Starting from now, the official mascot of Late Mate is 🦐

And last, but most important…

…thank you for being here. It means the world to me.

Dan 🦐