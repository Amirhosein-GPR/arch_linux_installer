use std::error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::process;
use std::thread;
use std::time;

const MAX_LINE_LENGTH: u8 = 64;
const INSTALLATION_STEPS_COUNT: u8 = 34;

enum PrintFormat {
    Bordered,
    DoubleDashedLine,
    DashedLine,
}

struct Question {
    answer: String,
}

impl Question {
    fn new() -> Self {
        Self {
            answer: String::new(),
        }
    }

    fn ask(&mut self, question: &str) {
        self.answer.clear();
        print!("{}", question);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut self.answer).unwrap();
        self.answer = self.answer.trim().to_string();
    }

    fn bool_ask(&mut self, question: &str) -> bool {
        loop {
            self.ask(format!("{question} (y/n): ").as_str());
            match self.answer.as_str() {
                "y" | "Y" => return true,
                "n" | "N" => return false,
                _ => {}
            }
        }
    }

    fn selecting_ask(&mut self, question: &str, choices: &[&str]) {
        loop {
            self.answer.clear();
            println!("{}\n", question);
            for (index, choice) in choices.into_iter().enumerate() {
                println!("{}. {choice}", index + 1);
            }
            print!("\nEnter number: ");
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut self.answer).unwrap();
            self.answer = self.answer.trim().to_string();
            if let Ok(num) = self.answer.parse::<u8>() {
                if num <= choices.len() as u8 && num > 0 {
                    break;
                }
            } else {
                println!("\nError: Enter only the number!\n");
            }
        }
    }
}

#[derive(Debug)]
enum AppError {
    InternalError(String),
    ExternalError(String),
}

impl error::Error for AppError {}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InternalError(internal_error) => {
                write!(f, "{}", internal_error)
            }
            Self::ExternalError(external_error) => {
                write!(f, "{}", external_error)
            }
        }
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::InternalError(value.to_string())
    }
}

struct AppConfig {
    uefi_install: bool,
    uefi_partition: Option<String>,
    boot_partition: Option<String>,
    root_partition: String,
    home_partition: Option<String>,
    username: String,
    encrypted_partitons: bool,
    swap_partition: Option<String>,
    current_installation_step: u8,
    total_installation_steps: u8,
}

impl AppConfig {
    fn new(total_installation_steps: u8) -> Self {
        Self {
            uefi_install: false,
            uefi_partition: None,
            boot_partition: None,
            root_partition: String::new(),
            home_partition: None,
            username: String::new(),
            encrypted_partitons: false,
            swap_partition: None,
            current_installation_step: 1,
            total_installation_steps,
        }
    }

    fn print_installation_status_and_save_config(&mut self, text: &str) {
        TextManager::set_color(TextColor::Cyan);
        let mut remaining_line_length = MAX_LINE_LENGTH - text.len() as u8;
        let mut individual_remaining_space = (remaining_line_length - 1) / 2;

        let mut format_string = (0..individual_remaining_space - 1)
            .map(|_i| "-")
            .collect::<String>();

        if remaining_line_length % 2 == 0 {
            println!("\n-{} {text} {}-", format_string, format_string);
        } else {
            println!("\n{} {text} {}-", format_string, format_string);
        }
        let empty_bordered_line = (0..MAX_LINE_LENGTH - 2).map(|_i| " ").collect::<String>();
        println!("|{}|", empty_bordered_line);

        let percentage = format!(
            "{}/{} | {}",
            self.current_installation_step,
            self.total_installation_steps,
            ((self.current_installation_step as f32 / self.total_installation_steps as f32) * 100.0)
                as u8
        );
        remaining_line_length = MAX_LINE_LENGTH - percentage.len() as u8;
        individual_remaining_space = (remaining_line_length - 1) / 2;

        format_string = (0..individual_remaining_space - 3)
            .map(|_i| "-")
            .collect::<String>();

        if remaining_line_length % 2 == 0 {
            println!("{}> [{percentage}%] <{}-\n", format_string, format_string);
        } else {
            println!("{}> [{percentage}%] <{}\n", format_string, format_string);
        }
        TextManager::reset_color_and_graphics();

        self.save_config();
    }

    fn save_config(&mut self) {
        let app_config_string = format!(
            "{}\n{:?}\n{:?}\n{}\n{:?}\n{}\n{}\n{:?}\n{}\n{}",
            self.uefi_install,
            self.uefi_partition,
            self.boot_partition,
            self.root_partition,
            self.home_partition,
            self.username,
            self.encrypted_partitons,
            self.swap_partition,
            self.current_installation_step,
            self.total_installation_steps
        );

        fs::write("./arch_linux_installer.conf", app_config_string)
            .expect("Error writing to ./arch_linux_installer.conf");
    }

    fn load_config(&mut self) -> Result<(), AppError> {
        let app_config_string = String::from_utf8(fs::read("./arch_linux_installer.conf")?).expect(
            "Error converting ./arch_linux_installer.conf contents to a valid UTF-8 string.",
        );

        let app_config_elements = app_config_string.split("\n").collect::<Vec<_>>();

        self.uefi_install = if app_config_elements[0] == "true" {
            true
        } else {
            false
        };
        self.uefi_partition = if app_config_elements[1] == "None" {
            None
        } else {
            Some(Self::extract_some_value(app_config_elements[1]))
        };
        self.boot_partition = if app_config_elements[2] == "None" {
            None
        } else {
            Some(Self::extract_some_value(app_config_elements[2]))
        };
        self.root_partition = app_config_elements[3].to_string();
        self.home_partition = if app_config_elements[4] == "None" {
            None
        } else {
            Some(Self::extract_some_value(app_config_elements[4]))
        };
        self.username = app_config_elements[5].to_string();
        self.encrypted_partitons = if app_config_elements[6] == "true" {
            true
        } else {
            false
        };
        self.swap_partition = if app_config_elements[7] == "None" {
            None
        } else {
            Some(Self::extract_some_value(app_config_elements[7]))
        };
        self.current_installation_step = app_config_elements[8]
            .parse()
            .expect("Error parsing string to u8");
        self.total_installation_steps = app_config_elements[9]
            .parse()
            .expect("Error parsing string to u8");

        Ok(())
    }

    fn remove_config(&self) {
        fs::remove_file("./arch_linux_installer.conf")
            .expect("Error removing ./arch_linux_installer.conf")
    }

    fn extract_some_value(some: &str) -> String {
        some.split("\"").collect::<Vec<_>>()[1].to_string()
    }

    fn reset(&mut self) {
        self.uefi_install = false;
        self.uefi_partition = None;
        self.boot_partition = None;
        self.root_partition = String::new();
        self.home_partition = None;
        self.username = String::new();
        self.encrypted_partitons = false;
        self.swap_partition = None;
        self.current_installation_step = 1;
    }
}

// Colors encoded in ANSI escape code
#[derive(Clone, Copy)]
enum TextColor {
    Reset,
    Black = 30,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default = 39,
}

impl fmt::Display for TextColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Clone, Copy)]
enum TextGraphics {
    Bold = 1,
    Dim,
    Italic,
    Underline,
    Blinking,
    Inverse = 7,
    Hidden,
    Strikethrough,
}

impl fmt::Display for TextGraphics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

struct TextManager;

impl TextManager {
    fn set_color(color: TextColor) {
        print!("\x1b[{color}m");
    }

    fn set_graphics(graphics: TextGraphics) {
        print!("\x1b[{graphics}m");
    }

    fn reset_color_and_graphics() {
        print!("\x1b[{}m", TextColor::Reset);
    }
}

enum OperationResult {
    Done,
    Error,
}

fn main() -> Result<(), AppError> {
    // Initializing question struct to use it in various parts of the program.
    let mut question = Question::new();

    print_welcome_message();

    if !question.bool_ask("Do you want to continue?") {
        return Ok(());
    }

    // Initializing app_config struct to use it in various parts of the program.
    let mut app_config = AppConfig::new(INSTALLATION_STEPS_COUNT);

    if let Ok(()) = app_config.load_config() {
        TextManager::set_color(TextColor::Yellow);
        formatted_print(
            "Aborted installation was detected",
            PrintFormat::DoubleDashedLine,
        );
        TextManager::reset_color_and_graphics();
        if !question.bool_ask(
            format!(
                "Do you want to continue installation from step ({}/{})?",
                app_config.current_installation_step, app_config.total_installation_steps
            )
            .as_str(),
        ) {
            app_config.reset();
        }
    }

    loop {
        match app_config.current_installation_step {
            1 => {
                app_config
                    .print_installation_status_and_save_config("BIOS / UEFI Installation mode");

                question.selecting_ask("Which installation mode do you want?", &["BIOS", "UEFI"]);
                if question.answer == "2" {
                    app_config.uefi_install = true;
                }

                print_operation_result(OperationResult::Done);
            }
            2 => {
                app_config.print_installation_status_and_save_config("Encrypted partitoins");

                if question.bool_ask("Do you want to encrypt your root and home partitions?") {
                    app_config.encrypted_partitons = true;
                }
            }
            3 => {
                app_config.print_installation_status_and_save_config("Configuring timedatectl");

                run_command("timedatectl", Some(&["set-ntp", "true"]))?;
                run_command("timedatectl", Some(&["status"]))?;

                print_operation_result(OperationResult::Done);
            }
            4 => {
                app_config.print_installation_status_and_save_config("Configuring partitions");

                run_command("fdisk", Some(&["-l"]))?;

                question.ask("Enter the disk you want to partion. (sda, sdb, ...): ");
                run_command(
                    "fdisk",
                    Some(&[format!("/dev/{}", question.answer).as_str()]),
                )?;

                println!("Partitioning results:\n");

                run_command("lsblk", None)?;

                print_operation_result(OperationResult::Done);
            }
            5 => {
                app_config.print_installation_status_and_save_config("Getting partition names");

                question.ask("Enter the name of your root partition: ");
                app_config.root_partition = question.answer.clone();

                if question.bool_ask("Do you have a separate boot partition?") {
                    question.ask("Enter the name of your boot partition: ");
                    app_config.boot_partition = Some(question.answer.clone());
                }

                if app_config.uefi_install {
                    question.ask("Enter the name of your uefi partition: ");
                    app_config.uefi_partition = Some(question.answer.clone());
                }

                if question.bool_ask("Do you have a separate home partition?") {
                    question.ask("Enter the name of your home partition: ");
                    app_config.home_partition = Some(question.answer.clone());
                }

                print_operation_result(OperationResult::Done);
            }
            6 => {
                app_config.print_installation_status_and_save_config("Formatting partitions");

                if question.bool_ask("Do you want to format your root partition?") {
                    if app_config.encrypted_partitons {
                        run_command(
                            "cryptsetup",
                            Some(&[
                                "luksFormat",
                                format!("/dev/{}", app_config.root_partition).as_str(),
                            ]),
                        )?;
                        run_command(
                            "cryptsetup",
                            Some(&[
                                "open",
                                format!("/dev/{}", app_config.root_partition).as_str(),
                                "cryptroot",
                            ]),
                        )?;
                        run_command("mkfs.btrfs", Some(&["-f", "/dev/mapper/cryptroot"]))?;
                    } else {
                        run_command(
                            "mkfs.btrfs",
                            Some(&["-f", format!("/dev/{}", app_config.root_partition).as_str()]),
                        )?;
                    }
                } else if app_config.encrypted_partitons {
                    run_command(
                        "cryptsetup",
                        Some(&[
                            "open",
                            format!("/dev/{}", app_config.root_partition).as_str(),
                            "cryptroot",
                        ]),
                    )?;
                }

                if let Some(boot_partition) = &app_config.boot_partition {
                    if question.bool_ask("Do you want to format your boot partition?") {
                        run_command(
                            "mkfs.btrfs",
                            Some(&["-f", format!("/dev/{}", boot_partition).as_str()]),
                        )?;
                    }
                }

                if let Some(uefi_partition) = &app_config.uefi_partition {
                    if question.bool_ask("Do you want to format your uefi partition?") {
                        run_command(
                            "mkfs.fat",
                            Some(&["-F32", format!("/dev/{}", uefi_partition).as_str()]),
                        )?;
                    }
                }

                if let Some(home_partition) = &app_config.home_partition {
                    if question.bool_ask("Do you want to format your home partition?") {
                        if app_config.encrypted_partitons {
                            run_command(
                                "cryptsetup",
                                Some(&["luksFormat", format!("/dev/{}", home_partition).as_str()]),
                            )?;
                            run_command(
                                "cryptsetup",
                                Some(&[
                                    "open",
                                    format!("/dev/{}", home_partition).as_str(),
                                    "crypthome",
                                ]),
                            )?;
                            run_command("mkfs.btrfs", Some(&["-f", "/dev/mapper/crypthome"]))?;
                        } else {
                            run_command(
                                "mkfs.btrfs",
                                Some(&["-f", format!("/dev/{}", home_partition).as_str()]),
                            )?;
                        }
                    } else if app_config.encrypted_partitons {
                        run_command(
                            "cryptsetup",
                            Some(&[
                                "open",
                                format!("/dev/{}", home_partition).as_str(),
                                "crypthome",
                            ]),
                        )?;
                    }
                }

                print_operation_result(OperationResult::Done);
            }
            7 => {
                app_config.print_installation_status_and_save_config("Enabling swap");

                if question.bool_ask("Do you want to enable swap?") {
                    question.ask("Enter name of the swap partition: ");
                    app_config.swap_partition = Some(question.answer.clone());

                    run_command(
                        "mkswap",
                        Some(&[format!("/dev/{}", question.answer).as_str()]),
                    )?;
                    run_command(
                        "swapon",
                        Some(&[format!("/dev/{}", question.answer).as_str()]),
                    )?;
                }

                print_operation_result(OperationResult::Done);
            }
            8 => {
                app_config.print_installation_status_and_save_config("Mounting partitions");

                if app_config.encrypted_partitons {
                    run_command("mount", Some(&["/dev/mapper/cryptroot", "/mnt"]))?;
                } else {
                    run_command(
                        "mount",
                        Some(&[
                            format!("/dev/{}", app_config.root_partition).as_str(),
                            "/mnt",
                        ]),
                    )?;
                }

                if let Some(boot_partition) = &app_config.boot_partition {
                    run_command("mkdir", Some(&["-p", "/mnt/boot"]))?;
                    run_command(
                        "mount",
                        Some(&[format!("/dev/{}", boot_partition).as_str(), "/mnt/boot"]),
                    )?;
                }

                if let Some(uefi_partition) = &app_config.uefi_partition {
                    run_command("mkdir", Some(&["-p", "/mnt/boot/EFI"]))?;
                    run_command(
                        "mount",
                        Some(&[format!("/dev/{}", uefi_partition).as_str(), "/mnt/boot/EFI"]),
                    )?;
                }

                if let Some(home_partition) = &app_config.home_partition {
                    run_command("mkdir", Some(&["-p", "/mnt/home"]))?;
                    if app_config.encrypted_partitons {
                        run_command("mount", Some(&["/dev/mapper/crypthome", "/mnt/home"]))?;
                    } else {
                        run_command(
                            "mount",
                            Some(&[format!("/dev/{}", home_partition).as_str(), "/mnt/home"]),
                        )?;
                    }
                }

                print_operation_result(OperationResult::Done);
            }
            9 => {
                app_config.print_installation_status_and_save_config("Updating mirrors");

                question.ask("Enter the name of your prefered country for mirrors. (For example: France,Germany,...): ");
                run_command(
                    "reflector",
                    Some(&[
                        "--latest",
                        "10",
                        "--country",
                        question.answer.as_str(),
                        "--protocol",
                        "http,https",
                        "--sort",
                        "rate",
                        "--save",
                        "/etc/pacman.d/mirrorlist",
                    ]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            10 => {
                app_config.print_installation_status_and_save_config("Configuring pacman");

                fs::write(
                    "/etc/pacman.conf",
                    fs::read_to_string("/etc/pacman.conf")
                        .expect("Error reading from /etc/pacman.conf")
                        .replace("#Color", "Color")
                        .replace("#VerbosePkgLists", "VerbosePkgLists")
                        .replace(
                            "#ParallelDownloads = 5",
                            "ParallelDownloads = 5\nILoveCandy",
                        ),
                )
                .expect("Error writing to /etc/pacman.conf");

                print_operation_result(OperationResult::Done);
            }
            11 => {
                app_config.print_installation_status_and_save_config(
                    "Starting to install base system and some softwares",
                );

                question.ask("What is your system's CPU brand? (Enter 'amd' or 'intel'): ");
                run_command(
                    "pacstrap",
                    Some(&[
                        "/mnt",
                        "base",
                        "linux",
                        "linux-firmware",
                        format!("{}-ucode", question.answer).as_str(),
                        "sudo",
                        "helix",
                        "grub",
                        "dosfstools",
                        "mtools",
                        "networkmanager",
                        "git",
                        "base-devel",
                    ]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            12 => {
                app_config
                    .print_installation_status_and_save_config("Generating file system table");

                let output = String::from_utf8(
                    process::Command::new("genfstab")
                        .args(["-U", "/mnt"])
                        .output()?
                        .stdout,
                )
                .expect("Error: Can't make string from vector of bytes.");

                fs::write("/mnt/etc/fstab", output).expect("Error writing to /mnt/etc/fstab");

                print_operation_result(OperationResult::Done);
            }
            13 => {
                app_config.print_installation_status_and_save_config(
                    "Configuring swap for encryption if necessary",
                );
                if app_config.encrypted_partitons {
                    if let Some(swap_partition) = &app_config.swap_partition {
                        run_command(
                            "swapoff",
                            Some(&[format!("/dev/{}", swap_partition).as_str()]),
                        )?;
                        run_command(
                            "mkfs.ext2",
                            Some(&[
                                "-L",
                                "cryptswap",
                                format!("/dev/{}", swap_partition).as_str(),
                                "1M",
                            ]),
                        )?;

                        let fstab_content = fs::read_to_string("/mnt/etc/fstab")
                            .expect("Error reading from /mnt/etc/fstab");
                        let found_swap_line = fstab_content
                            .lines()
                            .filter(|l| l.contains("swap"))
                            .collect::<Vec<&str>>()[0];
                        let swap_uuid =
                            found_swap_line.split_whitespace().collect::<Vec<&str>>()[0];

                        fs::write(
                            "/mnt/etc/fstab",
                            fstab_content.replace(swap_uuid, "/dev/mapper/swap"),
                        )
                        .expect("Error writing to /mnt/etc/fstab");
                    }
                }
                print_operation_result(OperationResult::Done);
            }
            14 => {
                app_config.print_installation_status_and_save_config(
                    "Configuring pacman for installed system",
                );

                fs::write(
                    "/mnt/etc/pacman.conf",
                    fs::read_to_string("/mnt/etc/pacman.conf")
                        .expect("Error reading from /mnt/etc/pacman.conf")
                        .replace("#Color", "Color")
                        .replace("#VerbosePkgLists", "VerbosePkgLists")
                        .replace(
                            "#ParallelDownloads = 5",
                            "ParallelDownloads = 5\nILoveCandy",
                        ),
                )
                .expect("Error writing to /mnt/etc/pacman.conf");

                print_operation_result(OperationResult::Done);
            }
            15 => {
                app_config.print_installation_status_and_save_config("Setting time zone");

                loop {
                    question.ask("Enter your time zone. (For example: Europe/London): ");
                    if !question.answer.contains("/") {
                        print_operation_result(OperationResult::Error);
                        if question.bool_ask("Please enter a forward slash (/) between the continent and city name. Do you want to enter the time zone again?") {
                    continue;
                } else {
                    TextManager::set_color(TextColor::Red);
                    formatted_print("Installation failed.", PrintFormat::Bordered);
                    return Err(AppError::InternalError(String::from("Error! Internal process exited. Setting time zone failed.")));
                }
                    }

                    break;
                }

                let time_zone_parts = question.answer.split("/").collect::<Vec<_>>();
                run_command(
                    "arch-chroot",
                    Some(&[
                        "/mnt",
                        "ln",
                        "-sf",
                        format!(
                            "/mnt/etc/usr/share/zoneinfo/{}/{}",
                            time_zone_parts[0], time_zone_parts[1]
                        )
                        .as_str(),
                        "/etc/localtime",
                    ]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            16 => {
                app_config.print_installation_status_and_save_config("Setting hardware clock");

                run_command("arch-chroot", Some(&["/mnt", "hwclock", "--systohc"]))?;

                print_operation_result(OperationResult::Done);
            }
            17 => {
                app_config.print_installation_status_and_save_config("Setting local");

                fs::write(
                    "/mnt/etc/locale.gen",
                    fs::read_to_string("/mnt/etc/locale.gen")
                        .expect("Error reading from /mnt/etc/locale.gen")
                        .replace("#en_US.UTF-8 UTF-8", "en_US.UTF-8 UTF-8"),
                )
                .expect("Error writing to /mnt/etc/locale.gen");

                run_command("arch-chroot", Some(&["/mnt", "locale-gen"]))?;

                print_operation_result(OperationResult::Done);
            }
            18 => {
                app_config.print_installation_status_and_save_config("Setting host name");

                question.ask("Enter your host name: ");
                fs::write("/mnt/etc/hostname", question.answer.clone())
                    .expect("Error writing to /mnt/etc/hostname");

                print_operation_result(OperationResult::Done);
            }
            19 => {
                app_config
                    .print_installation_status_and_save_config("Setting hosts configuaration");

                fs::write(
                    "/mnt/etc/hosts",
                    format!(
                        "127.0.0.1\tlocalhost\n::1 \t\tlocalhost\n127.0.1.1\t{}.localdomain\t{}",
                        question.answer, question.answer
                    ),
                )
                .expect("Error writing to /mnt/etc/hosts");

                print_operation_result(OperationResult::Done);
            }
            20 => {
                app_config.print_installation_status_and_save_config("Setting root pasword");

                loop {
                    if let Err(error) = run_command("arch-chroot", Some(&["/mnt", "passwd"])) {
                        print_operation_result(OperationResult::Error);
                        if question.bool_ask("Do you want to enter the root password again?") {
                            continue;
                        } else {
                            TextManager::set_color(TextColor::Red);
                            formatted_print("Installation failed.", PrintFormat::Bordered);
                            return Err(error);
                        }
                    } else {
                        break;
                    }
                }

                print_operation_result(OperationResult::Done);
            }
            21 => {
                app_config.print_installation_status_and_save_config("Creating user");

                loop {
                    question.ask("Enter your username: ");
                    if let Err(error) = run_command(
                        "arch-chroot",
                        Some(&["/mnt", "useradd", "-m", question.answer.as_str()]),
                    ) {
                        print_operation_result(OperationResult::Error);
                        if question.bool_ask("Do you want to enter the username again?") {
                            continue;
                        } else {
                            TextManager::set_color(TextColor::Red);
                            formatted_print("Installation failed.", PrintFormat::Bordered);
                            return Err(error);
                        }
                    } else {
                        break;
                    }
                }
                app_config.username = question.answer.clone();

                print_operation_result(OperationResult::Done);
            }
            22 => {
                app_config.print_installation_status_and_save_config("Setting your user pasword");

                loop {
                    if let Err(error) = run_command(
                        "arch-chroot",
                        Some(&["/mnt", "passwd", question.answer.as_str()]),
                    ) {
                        print_operation_result(OperationResult::Error);
                        if question.bool_ask("Do you want to enter the user password again?") {
                            continue;
                        } else {
                            TextManager::set_color(TextColor::Red);
                            formatted_print("Installation failed.", PrintFormat::Bordered);
                            return Err(error);
                        }
                    } else {
                        break;
                    }
                }

                print_operation_result(OperationResult::Done);
            }
            23 => {
                app_config.print_installation_status_and_save_config("Adding user to wheel group");

                run_command(
                    "arch-chroot",
                    Some(&["/mnt", "usermod", "-aG", "wheel", question.answer.as_str()]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            24 => {
                app_config.print_installation_status_and_save_config("Updating sudoers file");

                fs::write(
                    "/mnt/etc/sudoers",
                    fs::read_to_string("/mnt/etc/sudoers")
                        .expect("Error reading from /mnt/etc/sudoers")
                        .replace("# %wheel ALL=(ALL:ALL) ALL", "%wheel ALL=(ALL:ALL) ALL"),
                )
                .expect("Error writing to /mnt/etc/sudoers");

                print_operation_result(OperationResult::Done);
            }
            25 => {
                app_config.print_installation_status_and_save_config("Installing grub");

                if app_config.uefi_install {
                    run_command(
                        "arch-chroot",
                        Some(&["/mnt", "pacman", "-Sy", "efibootmgr", "--noconfirm"]),
                    )?;
                    run_command(
                        "arch-chroot",
                        Some(&[
                            "/mnt",
                            "grub-install",
                            "--target=x86_64-efi",
                            "--bootloader-id=grub_uefi",
                            "--recheck",
                        ]),
                    )?;
                } else {
                    question.ask("Enter your disk's name the Arch Linux has been installed to. (sda or sdb or ...): ");
                    run_command(
                        "arch-chroot",
                        Some(&[
                            "/mnt",
                            "grub-install",
                            "--target=i386-pc",
                            format!("/dev/{}", question.answer).as_str(),
                        ]),
                    )?;
                }

                print_operation_result(OperationResult::Done);
            }
            26 => {
                app_config.print_installation_status_and_save_config("Configuring grub");

                if question.bool_ask("Are you installing Arch Linux alongside Windows?") {
                    run_command(
                        "arch-chroot",
                        Some(&["/mnt", "pacman", "-Sy", "os-prober", "--noconfirm"]),
                    )?;

                    fs::write(
                        "/mnt/etc/default/grub",
                        fs::read_to_string("/mnt/etc/default/grub")
                            .expect("Error reading from /mnt/etc/default/grub")
                            .replace(
                                "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"",
                                "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3\"",
                            )
                            .replace(
                                "#GRUB_DISABLE_OS_PROBER=false",
                                "GRUB_DISABLE_OS_PROBER=false",
                            ),
                    )
                    .expect("Error writing to /mnt/etc/default/grub");
                } else {
                    fs::write(
                        "/mnt/etc/default/grub",
                        fs::read_to_string("/mnt/etc/default/grub")
                            .expect("Error reading from /mnt/etc/default/grub")
                            .replace(
                                "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"",
                                "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3\"",
                            )
                            .replace("GRUB_TIMEOUT=5", "GRUB_TIMEOUT=0"),
                    )
                    .expect("Error writing to /mnt/etc/default/grub");
                }

                if app_config.encrypted_partitons {
                    let root_uuid = find_uuid_in_blkid_command(&app_config.root_partition)?;
                    let cryptroot_uuid = find_uuid_in_blkid_command("cryptroot")?;

                    fs::write(
                "/mnt/etc/default/grub",
                fs::read_to_string("/mnt/etc/default/grub")
                    .expect("Error reading from /mnt/etc/default/grub")
                    .replace(
                        "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3\"",
                        format!(
                            "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 cryptdevice=UUID={}:cryptroot root=UUID={}\"",
                            root_uuid,
                            cryptroot_uuid
                        )
                        .as_str(),
                    )
                    .replace("GRUB_TIMEOUT=5", "GRUB_TIMEOUT=0"),
            )
            .expect("Error writing to /mnt/etc/default/grub");
                }

                print_operation_result(OperationResult::Done);
            }
            27 => {
                app_config.print_installation_status_and_save_config(
                    "Configuring and running mkinitcpio if necessary",
                );

                let has_nvidia_gpu = question.bool_ask("Do you have Nvidia GPU?");
                let has_intel_gpu = question.bool_ask("Do you have Intel GPU?");
                let mut writing_string = None;

                if has_nvidia_gpu {
                    run_command(
                        "arch-chroot",
                        Some(&["/mnt", "pacman", "-Sy", "nvidia", "--noconfirm"]),
                    )?;

                    writing_string = Some(["MODULES=()", "MODULES=(nvidia)"]);

                    if has_intel_gpu {
                        writing_string = Some(["MODULES=()", "MODULES=(i915 nvidia)"]);
                    }
                } else {
                    if has_intel_gpu {
                        writing_string = Some(["MODULES=()", "MODULES=(i915)"]);
                    }
                }

                if let Some(writing_string) = writing_string {
                    fs::write(
                        "/mnt/etc/mkinitcpio.conf",
                        fs::read_to_string("/mnt/etc/mkinitcpio.conf")
                            .expect("Error reading from /mnt/etc/mkinitcpio.conf")
                            .replace(writing_string[0], writing_string[1]),
                    )
                    .expect("Error writing to /mnt/etc/mkinitcpio.conf");
                    if app_config.encrypted_partitons {
                        fs::write(
                "/mnt/etc/mkinitcpio.conf",
                fs::read_to_string("/mnt/etc/mkinitcpio.conf")
                    .expect("Error reading from /mnt/etc/mkinitcpio.conf")
                    .replace("HOOKS=(base udev autodetect modconf kms keyboard keymap consolefont block filesystems fsck)", "HOOKS=(base udev autodetect modconf kms keyboard keymap consolefont block encrypt filesystems fsck)"),
            )
            .expect("Error writing to /mnt/etc/mkinitcpio.conf");
                    }

                    if let Err(error) =
                        run_command("arch-chroot", Some(&["/mnt", "mkinitcpio", "-p", "linux"]))
                    {
                        if !question.bool_ask(format!("{error}. This error occured in 'mkiniticpio -p linux' command which can be expected. Given this inforamtion, do you want to continue?").as_str()) {
                    TextManager::set_color(TextColor::Red);
                    formatted_print("Installation failed.", PrintFormat::Bordered);
                    return Err(error);
                }
                    }
                }

                print_operation_result(OperationResult::Done);
            }
            28 => {
                app_config.print_installation_status_and_save_config("Making grub config");

                run_command(
                    "arch-chroot",
                    Some(&["/mnt", "grub-mkconfig", "-o", "/boot/grub/grub.cfg"]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            29 => {
                app_config
                    .print_installation_status_and_save_config("Configuring crypttab if necessary");

                if app_config.encrypted_partitons {
                    if app_config.swap_partition.is_some() {
                        fs::write(
                            "/mnt/etc/crypttab",
                            fs::read_to_string("/mnt/etc/crypttab")
                                .expect("Error reading from /mnt/etc/crypttab")
                                .replace("# swap", "swap")
                                .replace("/dev/sdx4", "LABEL=cryptswap")
                                .replace("size=256", "size=256,offset=2048"),
                        )
                        .expect("Error writing to /mnt/etc/crypttab");
                    }

                    if let Some(home_partition) = &app_config.home_partition {
                        let mut file = OpenOptions::new()
                            .write(true)
                            .append(true)
                            .open("/mnt/etc/crypttab")
                            .expect("Error opening /mnt/etc/crypttab");

                        let home_uuid = find_uuid_in_blkid_command(&home_partition)?;

                        writeln!(file, "home UUID={} none", home_uuid)
                            .expect("Error writing to /mnt/etc/crypttab");
                    }
                }

                print_operation_result(OperationResult::Done);
            }
            30 => {
                app_config
                    .print_installation_status_and_save_config("Enabling network manager service");

                run_command(
                    "arch-chroot",
                    Some(&["/mnt", "systemctl", "enable", "NetworkManager"]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            31 => {
                app_config.print_installation_status_and_save_config(
                    "Installing KDE desktop and applications",
                );

                run_command(
                    "arch-chroot",
                    Some(&[
                        "/mnt",
                        "pacman",
                        "-Sy",
                        "sddm",
                        "bluedevil",
                        "breeze",
                        "breeze-gtk",
                        "kactivitymanagerd",
                        "kde-gtk-config",
                        "kgamma5",
                        "kpipewire",
                        "kscreen",
                        "kscreenlocker",
                        "ksystemstats",
                        "kwayland-integration",
                        "kwin",
                        "libkscreen",
                        "libksysguard",
                        "plasma-desktop",
                        "plasma-disks",
                        "plasma-firewall",
                        "plasma-nm",
                        "plasma-pa",
                        "plasma-systemmonitor",
                        "plasma-workspace",
                        "plasma-workspace-wallpapers",
                        "powerdevil",
                        "sddm-kcm",
                        "systemsettings",
                        "ark",
                        "dolphin",
                        "elisa",
                        "gwenview",
                        "kalarm",
                        "kcalc",
                        "kdeconnect",
                        "kdialog",
                        "konsole",
                        "ktimer",
                        "okular",
                        "partitionmanager",
                        "print-manager",
                        "spectacle",
                        "firefox",
                    ]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            32 => {
                app_config.print_installation_status_and_save_config("Enabling SDDM service");

                run_command(
                    "arch-chroot",
                    Some(&["/mnt", "systemctl", "enable", "sddm"]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            33 => {
                app_config.print_installation_status_and_save_config("Installing paru aur helper");
                println!("{}", format!("/home/{}", app_config.username).as_str());
                run_command(
                    "arch-chroot",
                    Some(&[
                        "-u",
                        app_config.username.as_str(),
                        "/mnt",
                        "git",
                        "clone",
                        "https://aur.archlinux.org/paru-bin.git",
                        format!("/home/{}/paru-bin", app_config.username).as_str(),
                    ]),
                )?;

                fs::write(
                    format!("/mnt/home/{}/makepkg.sh", app_config.username),
                    format!(
                        "#!/bin/bash\ncd /home/{}/paru-bin\nmakepkg -si",
                        app_config.username
                    ),
                )
                .expect(
                    format!(
                        "Error writing to /mnt/home/{}/makepkg.sh",
                        app_config.username
                    )
                    .as_str(),
                );

                run_command(
                    "arch-chroot",
                    Some(&[
                        "-u",
                        app_config.username.as_str(),
                        "/mnt",
                        "sudo",
                        "chmod",
                        "+x",
                        format!("/home/{}/makepkg.sh", app_config.username).as_str(),
                    ]),
                )?;
                run_command(
                    "arch-chroot",
                    Some(&[
                        "-u",
                        app_config.username.as_str(),
                        "/mnt",
                        format!("/home/{}/makepkg.sh", app_config.username).as_str(),
                    ]),
                )?;

                run_command(
                    "arch-chroot",
                    Some(&[
                        "/mnt",
                        "rm",
                        format!("/home/{}/makepkg.sh", app_config.username).as_str(),
                    ]),
                )?;

                run_command(
                    "arch-chroot",
                    Some(&[
                        "/mnt",
                        "rm",
                        "-r",
                        format!("/home/{}/paru-bin", app_config.username).as_str(),
                    ]),
                )?;

                print_operation_result(OperationResult::Done);
            }
            34 => {
                app_config.print_installation_status_and_save_config("Unmounting partition(s)");

                if let Some(uefi_partition) = &app_config.uefi_partition {
                    run_command(
                        "umount",
                        Some(&[format!("/dev/{}", uefi_partition).as_str()]),
                    )?;
                    println!("UEFI (/dev/{}): Unmounted", uefi_partition);
                }

                if let Some(boot_partition) = &app_config.boot_partition {
                    run_command(
                        "umount",
                        Some(&[format!("/dev/{}", boot_partition).as_str()]),
                    )?;
                    println!("Boot (/dev/{}): Unmounted", boot_partition);
                }

                if let Some(home_partition) = &app_config.home_partition {
                    if app_config.encrypted_partitons {
                        run_command("umount", Some(&["/dev/mapper/crypthome"]))?;
                        println!("Home (/dev/mapper/crypthome): Unmounted");
                        run_command("cryptsetup", Some(&["close", "/dev/mapper/crypthome"]))?;
                        println!("Home (/dev/mapper/crypthome): Closed");
                    } else {
                        run_command(
                            "umount",
                            Some(&[format!("/dev/{}", home_partition).as_str()]),
                        )?;
                        println!("Home (/dev/{}): Unmounted", home_partition);
                    }
                }

                if app_config.encrypted_partitons {
                    run_command("umount", Some(&["/dev/mapper/cryptroot"]))?;
                    println!("Root (/dev/mapper/cryptroot): Unmounted");
                    run_command("cryptsetup", Some(&["close", "/dev/mapper/cryptroot"]))?;
                    println!("Root (/dev/mapper/cryptroot): Closed");
                } else {
                    run_command(
                        "umount",
                        Some(&[format!("/dev/{}", app_config.root_partition).as_str()]),
                    )?;
                    println!("Root (/dev/{}): Unmounted", app_config.root_partition);
                }

                print_operation_result(OperationResult::Done);

                break;
            }
            _ => {
                panic!(
                    "Undefined step which is not in range: [1, {}]",
                    app_config.total_installation_steps
                );
            }
        }

        app_config.current_installation_step += 1;
    }

    // Printing successful installation message.
    {
        app_config.remove_config();

        TextManager::set_color(TextColor::Green);
        formatted_print("Installation finished successfully.", PrintFormat::Bordered);
        let mut second = 5;
        TextManager::reset_color_and_graphics();
        println!("\nSystem will restart in:\n");
        loop {
            if second == 0 {
                print!("{second}");
                break;
            }
            print!("{second}...");
            io::stdout().flush().unwrap();
            second -= 1;
            thread::sleep(time::Duration::from_secs(1));
        }
        TextManager::reset_color_and_graphics();

        run_command("reboot", None)?;
    }

    Ok(())
}

fn formatted_print(text: &str, format: PrintFormat) {
    let remaining_line_length = MAX_LINE_LENGTH - text.len() as u8;
    let individual_remaining_space = (remaining_line_length - 1) / 2;

    let format_string;
    match format {
        PrintFormat::Bordered => {
            format_string = (0..individual_remaining_space - 2)
                .map(|_i| " ")
                .collect::<String>();
        }
        PrintFormat::DoubleDashedLine => {
            format_string = (0..individual_remaining_space - 2)
                .map(|_i| "=")
                .collect::<String>();
        }
        PrintFormat::DashedLine => {
            format_string = (0..individual_remaining_space - 2)
                .map(|_i| "-")
                .collect::<String>();
        }
    }
    let empty_bordered_line = (0..MAX_LINE_LENGTH - 2).map(|_i| " ").collect::<String>();
    match format {
        PrintFormat::Bordered => {
            let full_line_string = (0..MAX_LINE_LENGTH).map(|_i| "=").collect::<String>();

            println!("{}", full_line_string);
            println!("|{}|", empty_bordered_line);
            if remaining_line_length % 2 == 0 {
                println!("| {} {text} {} |", format_string, format_string);
            } else {
                println!("|{} {text} {} |", format_string, format_string);
            }
            println!("|{}|", empty_bordered_line);
            println!("{}", full_line_string);
        }
        PrintFormat::DoubleDashedLine => {
            println!(" {} ", empty_bordered_line);
            if remaining_line_length % 2 == 0 {
                println!("=={} {text} {}==", format_string, format_string);
            } else {
                println!("={} {text} {}==", format_string, format_string);
            }
            println!(" {} ", empty_bordered_line);
        }
        PrintFormat::DashedLine => {
            println!(" {} ", empty_bordered_line);
            if remaining_line_length % 2 == 0 {
                println!("--{} {text} {}--", format_string, format_string);
            } else {
                println!("-{} {text} {}--", format_string, format_string);
            }
            println!(" {} ", empty_bordered_line);
        }
    }
}

fn run_command(command: &str, arguments: Option<&[&str]>) -> Result<(), AppError> {
    let exit_code;
    if let Some(arguments) = arguments {
        exit_code = process::Command::new(command)
            .args(arguments)
            .status()
            .unwrap()
            .code()
            .unwrap();
    } else {
        exit_code = process::Command::new(command)
            .status()
            .unwrap()
            .code()
            .unwrap();
    }

    if exit_code == 0 {
        Ok(())
    } else {
        Err(AppError::ExternalError(format!(
            "Error! External process exited with error code: {}",
            exit_code
        )))
    }
}

fn print_operation_result(operation_result: OperationResult) {
    match operation_result {
        OperationResult::Done => {
            TextManager::set_color(TextColor::Green);
            formatted_print("Done", PrintFormat::DashedLine);
        }
        OperationResult::Error => {
            TextManager::set_color(TextColor::Red);
            formatted_print("Error", PrintFormat::DashedLine);
        }
    }
    TextManager::reset_color_and_graphics();
}

fn find_uuid_in_blkid_command(partition_name: &str) -> Result<String, AppError> {
    let output = String::from_utf8(
        process::Command::new("arch-chroot")
            .args(["/mnt", "blkid"])
            .output()?
            .stdout,
    )
    .expect("Error: Can't make string from vector of bytes.");

    let output_lines = output.lines();
    let found_line = output_lines
        .filter(|l| l.contains(partition_name))
        .collect::<Vec<&str>>()[0];

    let line_segments = found_line.split_whitespace().collect::<Vec<&str>>();
    let mut partition_uuid = line_segments[1].split("=").collect::<Vec<&str>>()[1].to_string();

    partition_uuid.remove(0);
    partition_uuid.pop();

    Ok(partition_uuid.to_string())
}

fn print_welcome_message() {
    print!("\n\n\n\n\n\n\n\n\n\n");
    TextManager::set_color(TextColor::Red);
    formatted_print("Arch Linux install script", PrintFormat::Bordered);
    TextManager::set_color(TextColor::Green);
    formatted_print("(Version 0.1.9-alpha)", PrintFormat::DoubleDashedLine);
    TextManager::set_color(TextColor::Cyan);
    formatted_print("Made by Amirhosein_GPR", PrintFormat::Bordered);
    print!("\n\n\n\n\n\n\n\n\n\n");

    TextManager::set_color(TextColor::Magenta);
    formatted_print(
        format!("Total installation steps: {INSTALLATION_STEPS_COUNT}").as_str(),
        PrintFormat::DoubleDashedLine,
    );
    TextManager::reset_color_and_graphics();
}
