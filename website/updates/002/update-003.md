Update 003 — Houston, we have results!
---

Last week Dan sent me the PCB, and with a little soldering, we got our first prototype. It measures!

This is how it works. We take an app, like Kitty. Increase font size to some unreasonable size (for now, I hope you won’t need to do that).

Then we make our device pretend it’s a keyboard and produce USB events. It first “types” Spacebar and then Backspace:

![](../003/kitty.webp)

Then we position Late Mate Prototype Mark I (tm) measuring device on top of it:

![](../003/position.webp)

How? I’m glad you asked. Currently, like this:

![](../003/setup.jpeg)

Yes, it looks horrible. But it’s also surprisingly convenient! Until the first cracked screen, that is. So we will probably not include the clamp in the package.

Finally, we observe the brightness change. This gets us a graph like this:

![](../003/dell_normal.webp) 

Isn’t it beautiful? How smooth this curve is?

Don’t mind the UI, it’s a temporary quick-and-dirty tool (still better than Google Sheets) to analyze the raw stream of numbers coming from the device.

Here, let me annotate it for you:

![](../003/graph_one.webp) 

Already, we can say a few things:

- Delay between keypress and photons changing their intensity on a screen is about 42..44 ms for this particular case.
- LCD screen can’t change brightness immediately. It takes time, for this monitor about 16-17 ms actually, to go from black to white.

For funsies, I can turn on “Fast” response mode in the display’s settings and we can see how it starts to “overshoot”:

![](../003/dell_fast.webp)

Without improving latency much. Gives you a hell lot of artifacts, though. But now I can see them with science!

From here, we just do it a bunch of times, like 100, and we get a latency distribution graph! Like this:

![](../003/dell_kitty_top.webp)

From it you can tell that a combination of Kitty and Dell U3224KB react to keypress from 33 to 53 ms, averaging at 43 ms.

This is full end-to-end, so it includes USB polling, app code, drawing, compositor, GPU buffering, and even LCD switch times. But it’s also the latency you care about the most because that’s what your body and your brain actually experience.

Now to the actual results. We did a bunch of measurements and this is what we found (click to enlarge):

[![](../003/measurements.webp)](../003/measurements.webp)

Interesting observations:

- 120 Hz (Macbook Pro) doesn’t improve end-to-end latency that much. Maybe by 5-10 ms. Sounds good in isolation, but when you see that total latency starts at 40 ms, it only gives you around 20%...

- Latency at the top and the bottom of the screen differ significantly! For my screen, it’s ~10-15 ms! So instead of buying a 120 Hz screen, just move all your windows closer to the top :)

- Zed is not as fast as they [claim to be](https://twitter.com/nikitonsky/status/1771622685665116440). In typing latency, it’s closer to VS Code than to Sublime Text.

- Apple Terminal has two peaks about 40 ms apart. The reasons for that are unknown.

Another thing we learned is that the Macbook Pro's built-in XDR Display seems to modulate brightness. It’s not as smooth and constant as I get on my Dell, instead, it’s going up and down roughly every ms (or more often, we only sample at 2kHz):

![](../003/400_nits.webp)

Amplitude becomes smaller when you push brightness higher. The picture above was at ~400 nits, below is the full 1600 nits:

![](../003/1600_nits.webp)

But of course, you don’t run UI at 1600 nits. That presents us with a bit of a problem: how do you find a signal if your noise is like this:

![](../003/noise.jpeg)

(it’s hard to see but there’s a keypress in there).

Well, it can only mean one thing: we have something to work on! Subscribe for even more unexplainable results. And, eventually, pre-orders.

Bye-e!