<p align="center"><img src="https://user-images.githubusercontent.com/14863743/115953578-41e5a580-a4e4-11eb-84d9-45b296f9e18d.png" alt="Logo"/></p>

# ⚙️ gmpublisher

#### Currently in **Beta** development.

A powerful and feature-packed Workshop publisher for Garry's Mod is finally here!

[Click for downloads](https://github.com/WilliamVenner/gmpublisher/releases)

###### Are you a developer? You may also like my [VSCode GLua Enhanced](https://github.com/WilliamVenner/vscode-glua-enhanced) extension!

## Features

* Doesn't depend on gmad.exe or gmpublish.exe
* Publish & update your Workshop items
* Extract, search and browse GMA files and installed addons
* Bulk download & extract Workshop items and collections
* Upload animated GIFs as your Workshop item's icon
* Analyze which addons are taking up the most disk space using the addon size analyzer treemap
* (Coming soon) Manage Steam Workshop subscriptions
* (Coming soon) resource.AddWorkshop generator

## Languages

![](https://user-images.githubusercontent.com/14863743/115954244-ce459780-a4e7-11eb-9237-92eab7d17814.png) English

![](https://user-images.githubusercontent.com/14863743/115954306-195faa80-a4e8-11eb-8489-07ceca216211.png) French

![](https://user-images.githubusercontent.com/14863743/115954290-03ea8080-a4e8-11eb-86df-9001929981a7.png) German

![](https://user-images.githubusercontent.com/14863743/115954244-ce459780-a4e7-11eb-9237-92eab7d17814.png) Russian

![](https://user-images.githubusercontent.com/14863743/115954273-f7662800-a4e7-11eb-8196-551f0cc0e3f6.png) Polish

[Want to translate gmpublisher to your language?](https://github.com/WilliamVenner/gmpublisher/tree/master/i18n)

## Requirements

### Windows

[Webview2](https://go.microsoft.com/fwlink/p/?LinkId=2124703)

###### (In future this will not be a requirement)

### macOS, Linux

The program should work out-of-the-box.

## Technical Stuff

* The program makes heavy use of multithreading, and will work best on processors with a decent amount of cores.
* Made using [Rust](https://www.rust-lang.org/) (backend) and [Svelte](https://svelte.dev/) (frontend)
* This is not an Electron app; this is a [Tauri](https://github.com/tauri-apps/tauri) app. Big thanks to all the contributors to Tauri for their amazing work on finally killing Electron for good.
* gmpublisher uses the fantastic [steamworks-rs](https://crates.io/crates/steamworks) library for interfacing with the [Steamworks SDK](https://partner.steamgames.com/doc/api)
* The program is only about ~10 MB (which is probably just a lot of [panic unwinding traces](https://doc.rust-lang.org/nomicon/unwinding.html)!

## Media

![Screenshot](https://user-images.githubusercontent.com/14863743/115953601-5f1a7400-a4e4-11eb-831c-d6a924afbf33.png)

![Screenshot](https://user-images.githubusercontent.com/14863743/115953605-63469180-a4e4-11eb-9f96-90b992cbffc4.png)

![Screenshot](https://user-images.githubusercontent.com/14863743/115954341-5b88ec00-a4e8-11eb-8f27-c03d43df165a.png)

![Screenshot](https://user-images.githubusercontent.com/14863743/115953616-7c4f4280-a4e4-11eb-95c0-add80b1d41bd.png)

![Screenshot](https://user-images.githubusercontent.com/14863743/115953639-9db02e80-a4e4-11eb-935d-bad41cd30bde.png)

![Screenshot](https://user-images.githubusercontent.com/14863743/115953801-845bb200-a4e5-11eb-8fc2-8b142f2be237.png)

![Screenshot](https://user-images.githubusercontent.com/14863743/115953820-99d0dc00-a4e5-11eb-93a4-36e8b2248e87.png)

![Screenshot](https://user-images.githubusercontent.com/14863743/115953827-a35a4400-a4e5-11eb-9691-48e520eb9bb1.png)

![Screenshot](https://user-images.githubusercontent.com/14863743/115953670-bb7d9380-a4e4-11eb-8f54-f43fcd153d90.png)

<p align="center"><img src="https://i.imgur.com/Un4akZe.gif"/></p>
