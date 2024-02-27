-- Adapted from https://github.com/koreader/koreader/tree/master/plugins/hello.koplugin
local Dispatcher = require("dispatcher") -- luacheck:ignore
local InfoMessage = require("ui/widget/infomessage")
local UIManager = require("ui/uimanager")
local WidgetContainer = require("ui/widget/container/widgetcontainer")
local _ = require("gettext")

local WifiRemote = WidgetContainer:extend({
	name = "Wi-Fi Remote",
	is_doc_only = false,
})

function WifiRemote:onDispatcherRegisterActions()
	Dispatcher:registerAction(
		"wifiremote_toggle",
		{ category = "none", event = "ToggleWifiRemote", title = _("Wi-Fi Remote (toggle)"), general = true }
	)
end

function WifiRemote:init()
	self:onDispatcherRegisterActions()
	self.ui.menu:registerToMainMenu(self)
end

function WifiRemote:addToMainMenu(menu_items)
	menu_items.wifiremote_toggle = {
		text = _("Wi-Fi Remote (toggle)"),
		-- in which menu this should be appended
		sorting_hint = "tools",
		-- a callback when tapping
		callback = ToggleWifiRemote,
	}
end

function ToggleWifiRemote()
	-- Tried to use io.popen but it hangs when enabling the server (I assume due to the background process)
	os.execute("/opt/wifiremote/bin/wifiremote toggle > /tmp/wifiremote_toggle_status")
	local handle = io.open("/tmp/wifiremote_toggle_status", "r")
	if handle ~= nil then
		local output = handle:read("*a")
		local msg = output:gsub("\n$", "")
		UIManager:show(InfoMessage:new({
			text = _(msg),
			timeout = 2,
		}))
		handle:close()
	end
	os.remove("/tmp/wifiremote_toggle_status")
end

return WifiRemote
