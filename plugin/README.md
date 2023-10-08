# FLC Hints plugin for X-Plane 12

FLC Hints is a plugin for X-Plane 12 that displays a set of hint images for the current aircraft. Typically this is used
for checklists.

## Installation

FLC Hints is installed into the `plugins` directory of your X-Plane 12 installation. This directory is within
the `Resources` directory:

```
<...>/X-Plane 12/Resources/plugins
```

1. Download the latest release from
   the [FLC Hints GitHub releases page](https://github.com/flightlevelchange/hints/releases).
2. Extract the downloaded zip file into the `plugins` directory.

When installed correctly, the installation should look like this:

```
X-Plane 12
|- Resources
|  |- plugins
|  |  |- FLCHints
|  |  |  |- lin_x64
|  |  |  |  |- FLCHints.xpl
|  |  |  |- mac_x64
|  |  |  |  |- FLCHints.xpl
|  |  |  |- win_x64
|  |  |  |  |- FLCHints.xpl
```

## Usage

### Creating hints

Each aircraft has its own set of hints. These are stored as image files in a directory `hints` inside the aircraft
directory.

1. Create a directory called `hints` inside the aircraft, for
   example `<...>/X-Plane 12/Aircraft/Laminar Research/Cessna 172 SP/hints`
2. Add images to the `hints` directory. Supported image formats are JPEG and PNG, for example:

```
X-Plane 12
|- Aircraft
|  |- Laminar Research
|  |  |- Cessna 172 SP
|  |  |  |- hints
|  |  |  |  |- 001-preflight.png
|  |  |  |  |- 002-before-start.png
|  |  |  |  |- 003-starting.png
``` 

### Using hints

1. Start X-Plane 12
2. Load the aircraft
3. Open the hints window using the menu `Plugins > FLC Hints > Show hints` or the command `flc/hints/toggle`
4. Cycle hints with the mouse scroll-wheel or the commands `flc/hints/previous` and `flc/hints/next`


### Reloading hints

Hints can be reloaded from disk using the menu `Plugins > FLC Hints > Reload` or the command `flc/hints/reload`.

### Troubleshooting

If the plugin doesn't load or hints are not displayed, check the log file `X-Plane 12/Log.txt` for errors.
