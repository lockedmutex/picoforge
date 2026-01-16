use serde::Serialize;

mod error;
mod fido;
mod io;
mod logging;
mod rescue;
mod types;

// This will be temporary here untill moved to a dedicated module:

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
	pub is_maximized: bool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	logging::logger_init();
	log::info!("Initialisng PicoForge...");

	tauri::Builder::default()
		.plugin(tauri_plugin_shell::init())
		.plugin(tauri_plugin_opener::init())
		.invoke_handler(tauri::generate_handler![
			io::read_device_details,
			io::write_config,
			io::get_fido_info,
			io::change_fido_pin,
			io::get_credentials,
			io::delete_credential,
			io::set_min_pin_length,
			io::enable_secure_boot,
			io::reboot
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
