# Momentum

This project was generated with [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/#install-the-dioxus-cli) version 0.7.1

## Aim for this project

Ceate a program within which helps you keep track of your goal acheivement, health, wealth, research, and time.

![Health Picture](https://peggyosterkamp.com/wp-content/uploads/2014/04/Golden-Proportion-da-Vinci_02.jpg)

- Health 
	- integrates with schedule service (time aspect)
	- create workout / diet plans based on your case / goals, which then the program and you take into consideration how much time you can dedicate working out each day of the week, we can then derive your caloric expenditure
	-we can then feed your caloric expenditure into a diet algorithm (based on percent macro nutrients with diet presets *keto, vegan, etc*)
	-then based on that we can implement that information in your schedule as you see fit

![gamba](assets/README/gamba.gif)

- Wealth (FinCalc)
	- You put in income, expenses, and how much you'd like to divy up what's left and it does the rest!
	- I've done this kind of project [many times](https://github.com/wgauss/FinCalc) although I would like a proper implementation of a CRUD system rather than hard coding everything as i've done before out of laziness.


![Goal Picture](https://www.fossilconsulting.com/wp-content/uploads/2021/10/SMART-Graphic.png)

- Goal Acheivement
	- integrates with potentially every service
	- we essentially use the SMART goal template to ensure goal acheivement
	- depending on variables such as the type of goal it is, whether or not it needs time dedicated to acheive it or say we set aside x amount of money every paycheck to get x thing in x amount of time we can set scheduled reminders or goal trackers that we can see how far we've come to acheive said goal or see if we're on track to acheive it. 


![jaxBrain](https://images-ext-1.discordapp.net/external/etlizmJzNqsyLdcL9wRKGQSihAnUd9N08cAIXOj4KEI/https/i.imgur.com/FZFH9m0.png?format=webp&quality=lossless)
*jaxBrain has been a shelved project and I saw fit to revive it as this project is perfect to help spruce up the features of momentum*

- Research (jaxBrain)
	- still in design phase although the goal for this feature is to simplify and help visualize information for research purposes/notes
	- displays information in node trees, as at least to me information can have relationships to others that once reviewed can help simplify review based on the relational nature of information.

In terms of time there will be a timeline like aspect that i'd like to experiment with. of course the calender will still be available although I fancy the idea of handling my time in a horizontal fashion rather than the traditional calendar way.

May your days be better than yesterday, that's all we can work towards


# Development

Your new jumpstart project includes basic organization with an organized `assets` folder and a `components` folder.
If you chose to develop with the router feature, you will also have a `views` folder.

```
project/
├─ assets/ # Any assets that are used by the app should be placed here
├─ src/
│  ├─ main.rs # The entrypoint for the app. It also defines the routes for the app.
│  ├─ components/
│  │  ├─ mod.rs # Defines the components module
│  │  ├─ hero.rs # The Hero component for use in the home page
│  ├─ views/ # The views each route will render in the app.
│  │  ├─ mod.rs # Defines the module for the views route and re-exports the components for each route
│  │  ├─ blog.rs # The component that will render at the /blog/:id route
│  │  ├─ home.rs # The component that will render at the / route
├─ Cargo.toml # The Cargo.toml file defines the dependencies and feature flags for your project
```

### Automatic Tailwind (Dioxus 0.7+)

As of Dioxus 0.7, there no longer is a need to manually install tailwind. Simply `dx serve` and you're good to go!

Automatic tailwind is supported by checking for a file called `tailwind.css` in your app's manifest directory (next to Cargo.toml). To customize the file, use the dioxus.toml:

```toml
[application]
tailwind_input = "my.css"
tailwind_output = "assets/out.css"
```

### ( Rust && Dioxus ) || Visual Studio Required ( if == (((windows))) ) for Running Development Evironment for Contribution
goes without saying, but if you need a guide:

1:
``https://rust-lang.org/learn/get-started/``

2:
```bash
	cargo install dioxus-cli --version 0.7.0
```
## additional (((Windows))) caveats before step 2

install choco
Follow this guide: https://chocolatey.org/install#individual
install nasm
```bash
choco install nasm

then

Win + R "sysdm.cpl" , Advanced -> Enviroment Variables, System variables, Select Path, Edit, Create New -> " C:\Program Files\NASM\ "
```
install cmake
```bash
choco install cmake --installargs 'ADD_CMAKE_TO_PATH=System'
```


### IMPORTANT!
add this line at the top of ``src/main.rs``
```rust
#![windows_subsystem = "windows"]
```

### Serving Your App

Run the following command in the root of your project to start developing with the default platform:
 (it will serve desktop by default, will implement web version in the future)
```bash
dx serve
```

To run for a different platform, use the `--platform platform` flag. E.g.
```bash
dx serve --platform desktop
```

if for some reason on linux it doesn't display anything:
```bash
 export WEBKIT_DISABLE_DMABUF_RENDERER=1
 ```

 or you can include it in the command to run this like so:


 ```bash
 WEBKIT_DISABLE_DMABUF_RENDERER=1 dx serve
 ```