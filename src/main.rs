use std::error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::process;

const MAX_LINE_LENGTH: u8 = 64;
const INSTALLATION_STEPS_COUNT: u8 = 30;

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
        write!(f, "{}", self)
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
}

impl AppConfig {
    fn new() -> Self {
        Self {
            uefi_install: false,
            uefi_partition: None,
            boot_partition: None,
            root_partition: String::new(),
            home_partition: None,
            username: String::new(),
        }
    }
}

struct InstallationStatus {
    step: u8,
    total_steps: u8,
}

impl InstallationStatus {
    fn new(total_steps: u8) -> Self {
        Self {
            step: 0,
            total_steps,
        }
    }

    fn print(&mut self, text: &str) {
        ColorManager::set_color(Color::Blue);
        self.step += 1;

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

        let percentage = (((self.step as f32 / self.total_steps as f32) * 100.0) as u8).to_string();
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
        ColorManager::reset_color();
    }
}

// Colors encoded in ANSI escape code
#[derive(Clone, Copy)]
enum Color {
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

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

struct ColorManager;

impl ColorManager {
    fn set_color(color: Color) {
        print!("\x1b[0;{color}m");
    }

    fn reset_color() {
        print!("\x1b[0;{}m", Color::Default);
    }
}

enum OperationResult {
    Done,
    Error,
}

fn main() -> Result<(), AppError> {
    // Initializing question struct to use it in various parts of the program.
    let mut question = Question::new();
    // 0. Printing Welcome messages and asking user if he is sure to begin the process.
    {
        print!("\n\n\n\n\n\n\n\n\n\n");
        ColorManager::set_color(Color::Red);
        formatted_print("Arch Linux install script", PrintFormat::Bordered);
        ColorManager::set_color(Color::Green);
        formatted_print("(Version 0.1.2-alpha)", PrintFormat::DoubleDashedLine);
        ColorManager::set_color(Color::Blue);
        formatted_print("Made by Amirhosein_GPR", PrintFormat::Bordered);
        print!("\n\n\n\n\n\n\n\n\n\n");

        ColorManager::set_color(Color::Magenta);
        formatted_print(
            format!("Total installation steps: {INSTALLATION_STEPS_COUNT}").as_str(),
            PrintFormat::DoubleDashedLine,
        );
        ColorManager::reset_color();

        if !question.bool_ask("Do you want to continue?") {
            return Ok(());
        }
    }

    // Initializing app_config struct to use it in various parts of the program.
    let mut app_config = AppConfig::new();
    let mut installation_status = InstallationStatus::new(INSTALLATION_STEPS_COUNT);
    // Code set 1
    {
        installation_status.print("BIOS / UEFI Installation mode");

        question.selecting_ask("Which installation mode do you want?", &["BIOS", "UEFI"]);
        if question.answer == "2" {
            app_config.uefi_install = true;
        }

        print_operation_result(OperationResult::Done);
    }

    // Code set 2
    {
        installation_status.print("Configuring timedatectl");

        run_command("timedatectl", Some(&["set-ntp", "true"]))?;
        run_command("timedatectl", Some(&["status"]))?;

        print_operation_result(OperationResult::Done);
    }

    // Code set 3
    {
        installation_status.print("Configuring partitions");

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

    // Code set 4
    {
        installation_status.print("Getting partition names");

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

    // Code set 5
    {
        installation_status.print("Formatting partitions");

        if question.bool_ask("Do you want to format your root partition?") {
            run_command(
                "mkfs.btrfs",
                Some(&["-f", format!("/dev/{}", app_config.root_partition).as_str()]),
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
                run_command(
                    "mkfs.btrfs",
                    Some(&["-f", format!("/dev/{}", home_partition).as_str()]),
                )?;
            }
        }

        print_operation_result(OperationResult::Done);
    }

    // Code set 6
    {
        installation_status.print("Enabling swap");

        if question.bool_ask("Do you want to enable swap?") {
            question.ask("Enter name of the swap partition: ");

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

    // Code set 7
    {
        installation_status.print("Mounting partitions");

        run_command(
            "mount",
            Some(&[
                format!("/dev/{}", app_config.root_partition).as_str(),
                "/mnt",
            ]),
        )?;

        if let Some(boot_partition) = app_config.boot_partition {
            run_command("mkdir", Some(&["-p", "/mnt/boot"]))?;
            run_command(
                "mount",
                Some(&[format!("/dev/{}", boot_partition).as_str(), "/mnt/boot"]),
            )?;
        }

        if let Some(uefi_partition) = app_config.uefi_partition {
            run_command("mkdir", Some(&["-p", "/mnt/boot/EFI"]))?;
            run_command(
                "mount",
                Some(&[format!("/dev/{}", uefi_partition).as_str(), "/mnt/boot/EFI"]),
            )?;
        }

        if let Some(home_partition) = app_config.home_partition {
            run_command("mkdir", Some(&["-p", "/mnt/home"]))?;
            run_command(
                "mount",
                Some(&[format!("/dev/{}", home_partition).as_str(), "/mnt/home"]),
            )?;
        }

        print_operation_result(OperationResult::Done);
    }

    // Code set 8
    {
        installation_status.print("Updating mirrors");

        question.ask(
        "Enter the name of your prefered country for mirrors. (For example: France,Germany,...): ",
    );
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

    // Code set 9
    {
        installation_status.print("Configuring pacman");

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

    // Code set 10
    {
        installation_status.print("Starting to install base system and some softwares");

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

    // Code set 11
    {
        installation_status.print("Generating file system table");

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

    // Code set 12
    {
        installation_status.print("Configuring pacman for installed system");

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

    // Code set 13
    {
        installation_status.print("Setting time zone");

        loop {
            question.ask("Enter your time zone. (For example: Europe/London): ");
            if !question.answer.contains("/") {
                print_operation_result(OperationResult::Error);
                if question.bool_ask("Please enter a forward slash (/) between the continent and city name. Do you want to enter the time zone again?") {
                    continue;
                } else {
                    ColorManager::set_color(Color::Red);
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

    // Code set 14
    {
        installation_status.print("Setting hardware clock");

        run_command("arch-chroot", Some(&["/mnt", "hwclock", "--systohc"]))?;

        print_operation_result(OperationResult::Done);
    }

    // Code set 15
    {
        installation_status.print("Setting local");

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

    // Code set 16
    {
        installation_status.print("Setting host name");

        question.ask("Enter your host name: ");
        fs::write("/mnt/etc/hostname", question.answer.clone())
            .expect("Error writing to /mnt/etc/hostname");

        print_operation_result(OperationResult::Done);
    }

    // Code set 17
    {
        installation_status.print("Setting hosts configuaration");

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

    // Code set 18
    {
        installation_status.print("Setting root pasword");

        loop {
            if let Err(error) = run_command("arch-chroot", Some(&["/mnt", "passwd"])) {
                print_operation_result(OperationResult::Error);
                if question.bool_ask("Do you want to enter the root password again?") {
                    continue;
                } else {
                    ColorManager::set_color(Color::Red);
                    formatted_print("Installation failed.", PrintFormat::Bordered);
                    return Err(error);
                }
            } else {
                break;
            }
        }

        print_operation_result(OperationResult::Done);
    }

    // Code set 19
    {
        installation_status.print("Creating user");

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
                    ColorManager::set_color(Color::Red);
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

    // Code set 20
    {
        installation_status.print("Setting your user pasword");

        loop {
            if let Err(error) = run_command(
                "arch-chroot",
                Some(&["/mnt", "passwd", question.answer.as_str()]),
            ) {
                print_operation_result(OperationResult::Error);
                if question.bool_ask("Do you want to enter the user password again?") {
                    continue;
                } else {
                    ColorManager::set_color(Color::Red);
                    formatted_print("Installation failed.", PrintFormat::Bordered);
                    return Err(error);
                }
            } else {
                break;
            }
        }

        print_operation_result(OperationResult::Done);
    }

    // Code set 21
    {
        installation_status.print("Adding user to wheel group");

        run_command(
            "arch-chroot",
            Some(&["/mnt", "usermod", "-aG", "wheel", question.answer.as_str()]),
        )?;

        print_operation_result(OperationResult::Done);
    }

    // Code set 22
    {
        installation_status.print("Updating sudoers file");

        fs::write(
            "/mnt/etc/sudoers",
            fs::read_to_string("/mnt/etc/sudoers")
                .expect("Error reading from /mnt/etc/sudoers")
                .replace("# %wheel ALL=(ALL:ALL) ALL", "%wheel ALL=(ALL:ALL) ALL"),
        )
        .expect("Error writing to /mnt/etc/sudoers");

        print_operation_result(OperationResult::Done);
    }

    // Code set 23
    {
        installation_status.print("Installing grub");

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
            question.ask(
            "Enter your disk's name the Arch Linux has been installed to. (sda or sdb or ...): ",
        );
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

    // Code set 24
    {
        installation_status.print("Configuring grub");

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
                    ),
            )
            .expect("Error writing to /mnt/etc/default/grub");
        }

        print_operation_result(OperationResult::Done);
    }

    // Code set 25
    {
        installation_status.print("Configuring and running mkinitcpio if necessary");

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

            if let Err(error) =
                run_command("arch-chroot", Some(&["/mnt", "mkinitcpio", "-p", "linux"]))
            {
                if !question.bool_ask(format!("{error}. This error occured in 'mkiniticpio -p linux' command which can be expected. Given this inforamtion, do you want to continue?").as_str()) {
                    ColorManager::set_color(Color::Red);
                    formatted_print("Installation failed.", PrintFormat::Bordered);
                    return Err(error);
                }
            }
        }

        print_operation_result(OperationResult::Done);
    }

    // Code set 26
    {
        installation_status.print("Making grub config");

        run_command(
            "arch-chroot",
            Some(&["/mnt", "grub-mkconfig", "-o", "/boot/grub/grub.cfg"]),
        )?;

        print_operation_result(OperationResult::Done);
    }

    // Code set 27
    {
        installation_status.print("Enabling network manager service");

        run_command(
            "arch-chroot",
            Some(&["/mnt", "systemctl", "enable", "NetworkManager"]),
        )?;

        print_operation_result(OperationResult::Done);
    }

    // Code set 28
    {
        installation_status.print("Installing KDE desktop and applications");

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
                "konsole",
                "ktimer",
                "okular",
                "partitionmanager",
                "print-manager",
                "spectacle",
                "--noconfirm",
            ]),
        )?;

        print_operation_result(OperationResult::Done);
    }

    // Code set 29
    {
        installation_status.print("Enabling SDDM service");

        run_command(
            "arch-chroot",
            Some(&["/mnt", "systemctl", "enable", "sddm"]),
        )?;

        print_operation_result(OperationResult::Done);
    }

    // Code set 30
    {
        installation_status.print("Installing paru aur helper");

        run_command(
            "arch-chroot",
            Some(&[
                "-u",
                app_config.username.as_str(),
                "/mnt",
                "cd",
                format!("/home/{};", app_config.username).as_str(),
                "git",
                "clone",
                "https://aur.archlinux.org/paru-bin.git",
            ]),
        )?;
        run_command(
            "arch-chroot",
            Some(&[
                "-u",
                app_config.username.as_str(),
                "/mnt",
                "cd",
                format!("/home/{}/paru-bin;", app_config.username).as_str(),
                "makepkg",
                "-si",
            ]),
        )?;

        print_operation_result(OperationResult::Done);
    }

    // Printing 'Installation finished' message.
    {
        ColorManager::set_color(Color::Cyan);
        formatted_print(
            "Installation finished successfully. You can now reboot.",
            PrintFormat::Bordered,
        );
        ColorManager::reset_color();
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
            ColorManager::set_color(Color::Green);
            formatted_print("Done", PrintFormat::DashedLine);
        }
        OperationResult::Error => {
            ColorManager::set_color(Color::Red);
            formatted_print("Error", PrintFormat::DashedLine);
        }
    }
    ColorManager::reset_color();
}
