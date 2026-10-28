<script module lang="ts">
  export type IconName =
    | 'cpu'
    | 'ram'
    | 'network'
    | 'battery'
    | 'bolt'
    | 'volume'
    | 'mute'
    | 'keyboard'
    | 'tab'
    | 'external'
    | 'enter'
    | 'win'
    | 'caps'
    | 'download'
    | 'bell'
    | 'media'
    | 'activity'
    | 'settings'
    | 'disk'
    | 'bluetooth'
    | 'moon'
    | 'monitor'
    | 'camera'
    | 'close'
    | 'play'
    | 'pause'
    | 'prev'
    | 'next'
    | 'microphone'
    | 'sun'
    | 'timer'
    | 'focus'
    | 'wifi'
    | 'palette'
    | 'sliders'
    | 'plug'
    | 'eye'
    | 'sparks'
    | 'clock'
    | 'align-left'
    | 'align-center'
    | 'align-right'
    | 'folder'
    | 'apps'
    | 'plus'
    | 'check'
    | 'layout-rail'
    | 'layout-tabs'
    | 'trash'
    | 'edit';
</script>

<script lang="ts">
  import cpu from 'iconoir/icons/cpu.svg?raw';
  import electronicsChip from 'iconoir/icons/electronics-chip.svg?raw';
  import network from 'iconoir/icons/network.svg?raw';
  import battery75 from 'iconoir/icons/battery-75.svg?raw';
  import flash from 'iconoir/icons/flash.svg?raw';
  import soundHigh from 'iconoir/icons/sound-high.svg?raw';
  import soundOff from 'iconoir/icons/sound-off.svg?raw';
  import key from 'iconoir/icons/key.svg?raw';
  import newTab from 'iconoir/icons/new-tab.svg?raw';
  import openNewWindow from 'iconoir/icons/open-new-window.svg?raw';
  import cornerBottomLeft from 'iconoir/icons/corner-bottom-left.svg?raw';
  import windows from 'iconoir/icons/windows.svg?raw';
  import arrowUp from 'iconoir/icons/arrow-up.svg?raw';
  import download from 'iconoir/icons/download.svg?raw';
  import bell from 'iconoir/icons/bell.svg?raw';
  import bellOff from 'iconoir/icons/bell-off.svg?raw';
  import musicDoubleNote from 'iconoir/icons/music-double-note.svg?raw';
  import activity from 'iconoir/icons/activity.svg?raw';
  import settings from 'iconoir/icons/settings.svg?raw';
  import hardDrive from 'iconoir/icons/hard-drive.svg?raw';
  import bluetooth from 'iconoir/icons/bluetooth.svg?raw';
  import halfMoon from 'iconoir/icons/half-moon.svg?raw';
  import computer from 'iconoir/icons/computer.svg?raw';
  import camera from 'iconoir/icons/camera.svg?raw';
  import xmark from 'iconoir/icons/xmark.svg?raw';
  import microphone from 'iconoir/icons/microphone.svg?raw';
  import sunLight from 'iconoir/icons/sun-light.svg?raw';
  import timer from 'iconoir/icons/timer.svg?raw';
  import palette from 'iconoir/icons/palette.svg?raw';
  import controlSlider from 'iconoir/icons/control-slider.svg?raw';
  import evPlug from 'iconoir/icons/ev-plug.svg?raw';
  import eye from 'iconoir/icons/eye.svg?raw';
  import sparks from 'iconoir/icons/sparks.svg?raw';
  import clock from 'iconoir/icons/clock.svg?raw';
  import alignLeft from 'iconoir/icons/align-left.svg?raw';
  import alignCenter from 'iconoir/icons/align-center.svg?raw';
  import alignRight from 'iconoir/icons/align-right.svg?raw';
  import playSolid from 'iconoir/icons/play-solid.svg?raw';
  import pauseSolid from 'iconoir/icons/pause-solid.svg?raw';
  import skipPrevSolid from 'iconoir/icons/skip-prev-solid.svg?raw';
  import skipNextSolid from 'iconoir/icons/skip-next-solid.svg?raw';
  import folder from 'iconoir/icons/folder.svg?raw';
  import wifi from 'iconoir/icons/wifi.svg?raw';
  import appStore from 'iconoir/icons/app-store.svg?raw';
  import plus from 'iconoir/icons/plus.svg?raw';
  import check from 'iconoir/icons/check.svg?raw';
  import layoutLeft from 'iconoir/icons/layout-left.svg?raw';
  import menu from 'iconoir/icons/menu.svg?raw';
  import edit from 'iconoir/icons/edit.svg?raw';
  import trash from 'iconoir/icons/trash.svg?raw';

  // Iconoir ships complete documents; keep only the inner markup for our own <svg>.
  const inner = (svg: string) => svg.slice(svg.indexOf('>') + 1, svg.lastIndexOf('</svg>'));

  const glyphs: Record<IconName, string> = {
    cpu: inner(cpu),
    ram: inner(electronicsChip),
    network: inner(network),
    battery: inner(battery75),
    bolt: inner(flash),
    volume: inner(soundHigh),
    mute: inner(soundOff),
    keyboard: inner(key),
    tab: inner(newTab),
    external: inner(openNewWindow),
    enter: inner(cornerBottomLeft),
    win: inner(windows),
    caps: inner(arrowUp),
    download: inner(download),
    bell: inner(bell),
    media: inner(musicDoubleNote),
    activity: inner(activity),
    settings: inner(settings),
    disk: inner(hardDrive),
    bluetooth: inner(bluetooth),
    moon: inner(halfMoon),
    monitor: inner(computer),
    camera: inner(camera),
    close: inner(xmark),
    play: inner(playSolid),
    pause: inner(pauseSolid),
    prev: inner(skipPrevSolid),
    next: inner(skipNextSolid),
    microphone: inner(microphone),
    sun: inner(sunLight),
    timer: inner(timer),
    focus: inner(bellOff),
    palette: inner(palette),
    sliders: inner(controlSlider),
    plug: inner(evPlug),
    eye: inner(eye),
    sparks: inner(sparks),
    clock: inner(clock),
    'align-left': inner(alignLeft),
    'align-center': inner(alignCenter),
    'align-right': inner(alignRight),
    folder: inner(folder),
    wifi: inner(wifi),
    apps: inner(appStore),
    plus: inner(plus),
    check: inner(check),
    'layout-rail': inner(layoutLeft),
    'layout-tabs': inner(menu),
    edit: inner(edit),
    trash: inner(trash),
  };

  let {
    name,
    class: className = 'size-3.5',
    style = '',
  }: { name: IconName; class?: string; style?: string } = $props();
</script>

<svg
  class="{className} shrink-0"
  {style}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {@html glyphs[name]}
</svg>
