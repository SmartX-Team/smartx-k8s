bluez_monitor.properties = {
  -- Enable the logind module, which arbitrates which user will be allowed
  -- to have bluetooth audio enabled at any given time (particularly useful
  -- if you are using GDM as a display manager, as the gdm user also launches
  -- pipewire and wireplumber).
  -- This requires access to the D-Bus user session; disable if you are running
  -- a system-wide instance of wireplumber.
  ["with-logind"] = false,
}
