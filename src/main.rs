use std::error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::process;

const MAX_LINE_LENGTH: u8 = 64;
const INSTALLATION_STEPS_COUNT: u8 = 29;

enum PrintFormat {
    Bordered,
    DoubleDashedLine,
    DashedLine,
}

struct Question {
    answer: String,
}

#[derive(Debug)]
enum AppError {
    ExternalError(String),
}

struct AppConfig {
    uefi_install: bool,
    uefi_partition: Option<String>,
    boot_partition: Option<String>,
    root_partition: String,
    home_partition: Option<String>,
    username: String,
}

struct InstallationStatus {
    step: u8,
    total_steps: u8,
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

impl error::Error for AppError {}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
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
            if let Ok(_num) = self.answer.parse::<u8>() {
                break;
            } else {
                println!("\nError: Enter only the number!\n");
            }
        }
    }
}

impl InstallationStatus {
    fn new(total_steps: u8) -> Self {
        Self {
            step: 0,
            total_steps,
        }
    }

    fn print(&mut self, text: &str) {
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
    }
}

fn main() -> Result<(), AppError> {
    // Initializing question struct to use it in various parts of the program.
    let mut question = Question::new();
    // 0. Printing Welcome messages and asking user if he is sure to begin the process.
    {
        print!("\n\n\n\n\n\n\n\n\n\n");
        formatted_print("Arch Linux install script", PrintFormat::Bordered);
        formatted_print("(Version 0.1.1-alpha)", PrintFormat::DoubleDashedLine);
        formatted_print("Made by Amirhosein_GPR", PrintFormat::Bordered);
        print!("\n\n\n\n\n\n\n\n\n\n");

        formatted_print(
            format!("Total installation steps: {INSTALLATION_STEPS_COUNT}").as_str(),
            PrintFormat::DoubleDashedLine,
        );

        if !question.bool_ask("Do you want to continue?") {
            return Ok(());
        }
    }

    // Initializing app_config struct to use it in various parts of the program.
    let mut app_config = AppConfig::new();
    let mut installation_status = InstallationStatus::new(INSTALLATION_STEPS_COUNT);
    // Command set 1
    {
        installation_status.print("BIOS / UEFI Installation mode");

        question.selecting_ask("Which installation mode do you want?", &["BIOS", "UEFI"]);
        if question.answer == "2" {
            app_config.uefi_install = true;
        }
    }

    // Command set 2
    {
        installation_status.print("Configuring timedatectl");

        run_command("timedatectl", Some(&["set-ntp", "true"]))?;
        run_command("timedatectl", Some(&["status"]))?;
    }

    // Command set 3
    {
        installation_status.print("Configuring partitions");

        run_command("fdisk", Some(&["-l"]))?;

        question.ask("Enter the disk you want to partion. (sda, sdb, ...): ");
        run_command(
            "fdisk",
            Some(&[format!("/dev/{}", question.answer).as_str()]),
        )?;

        println!("Partitioning results:");

        run_command("lsblk", None)?;
    }

    // Command set 4
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
    }

    // Command set 5
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
    }

    // Command set 6
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
    }

    // Command set 7
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
            run_command("mkdir", Some(&["/mnt/boot"]))?;
            run_command(
                "mount",
                Some(&[format!("/dev/{}", boot_partition).as_str(), "/mnt/boot"]),
            )?;
        }

        if let Some(uefi_partition) = app_config.uefi_partition {
            run_command("mkdir", Some(&["/mnt/boot/EFI"]))?;
            run_command(
                "mount",
                Some(&[format!("/dev/{}", uefi_partition).as_str(), "/mnt/boot/EFI"]),
            )?;
        }

        if let Some(home_partition) = app_config.home_partition {
            run_command("mkdir", Some(&["/mnt/home"]))?;
            run_command(
                "mount",
                Some(&[format!("/dev/{}", home_partition).as_str(), "/mnt/home"]),
            )?;
        }
    }

    // Command set 8
    {
        installation_status.print("Updating mirrors");

        question.ask(
        "Enter the name of your prefered country for mirrirs. (Like this: France,Germany,...) : ",
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
    }

    // Command set 9
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
    }

    // Command set 10
    {
        installation_status.print("Generating file system table");

        run_command("genfstab", Some(&["-U", "/mnt", ">", "/mnt/etc/fstab"]))?;
    }

    // Command set 11
    {
        installation_status.print("Changing current root to the installed system root");

        run_command("arch-chroot", Some(&["/mnt"]))?;
    }

    // Command set 12
    {
        installation_status.print("Setting time zone");

        run_command(
            "ln",
            Some(&[
                "-sf",
                "/mnt/etc/usr/share/zoneinfo/Asia/Tehran",
                "/etc/localtime",
            ]),
        )?;
    }

    // Command set 13
    {
        installation_status.print("Setting hardware clock");

        run_command("hwclock", Some(&["--systohc"]))?;
    }

    // Command set 14
    {
        installation_status.print("Setting local");

        fs::write(
            "/etc/locale.gen",
            fs::read_to_string("/etc/locale.gen")
                .expect("Error reading from /etc/locale.gen")
                .replace("#en_US.UTF-8 UTF-8", "en_US.UTF-8 UTF-8"),
        )
        .expect("Error writing to /etc/locale.gen");

        run_command("locale-gen", None)?;
    }

    // Command set 15
    {
        installation_status.print("Setting host name");

        question.ask("Enter your host name");
        fs::write("/etc/hostname", question.answer.clone())
            .expect("Error writing to /etc/hostname");
    }

    // Command set 16
    {
        installation_status.print("Setting hosts configuaration");

        fs::write(
            "/etc/hosts",
            format!(
                "127.0.0.1\tlocalhost\n::1 \t\tlocalhost\n127.0.1.1\t{}.localdomain\t{}",
                question.answer, question.answer
            ),
        )
        .expect("Error writing to /etc/hosts");
    }

    // Command set 17
    {
        installation_status.print("Setting root pasword");

        run_command("passwd", None)?;
    }

    // Command set 18
    {
        installation_status.print("Creating user");

        question.ask("Enter your username: ");
        run_command("useradd", Some(&["-m", question.answer.as_str()]))?;
        app_config.username = question.answer.clone();
    }

    // Command set 19
    {
        installation_status.print("Setting your user pasword");

        run_command("passwd", Some(&[question.answer.as_str()]))?;
    }

    // Command set 20
    {
        installation_status.print("Adding user to wheel group");

        run_command("usermod", Some(&["-aG", "wheel", question.answer.as_str()]))?;
    }

    // Command set 21
    {
        installation_status.print("Updating sudoers file");

        fs::write(
            "/etc/sudoers",
            fs::read_to_string("/etc/sudoers")
                .expect("Error reading from /etc/sudoers")
                .replace("# %wheel ALL=(ALL:ALL) ALL", "%wheel ALL=(ALL:ALL) ALL"),
        )
        .expect("Error writing to /etc/sudoers");
    }

    // Command set 22
    {
        installation_status.print("Installing grub");

        if app_config.uefi_install {
            run_command("pacman", Some(&["-Sy", "efibootmgr"]))?;
            run_command(
                "grub-install",
                Some(&[
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
                "grub-install",
                Some(&[
                    "--target=i386-pc",
                    format!("/dev/{}", question.answer).as_str(),
                ]),
            )?;
        }
    }

    // Command set 23
    {
        installation_status.print("Configuring grub");

        if question.bool_ask("Are you installing Arch Linux alongside Windows?") {
            run_command("pacman", Some(&["-Sy", "os-prober"]))?;

            fs::write(
                "/etc/default/grub",
                fs::read_to_string("/etc/default/grub")
                    .expect("Error reading from /etc/default/grub")
                    .replace(
                        "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"",
                        "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3\"",
                    )
                    .replace(
                        "#GRUB_DISABLE_OS_PROBER=false",
                        "GRUB_DISABLE_OS_PROBER=false",
                    ),
            )
            .expect("Error writing to /etc/default/grub");
        } else {
            fs::write(
                "/etc/default/grub",
                fs::read_to_string("/etc/default/grub")
                    .expect("Error reading from /etc/default/grub")
                    .replace(
                        "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"",
                        "GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3\"",
                    ),
            )
            .expect("Error writing to /etc/default/grub");
        }
    }

    // Command set 24
    {
        installation_status.print("Configuring and running mkinitcpio if necessary");

        let has_nvidia_gpu = question.bool_ask("Do you have Nvidia GPU?");
        let has_intel_gpu = question.bool_ask("Do you have Intel GPU?");
        let mut writing_string = None;

        if has_nvidia_gpu {
            run_command("pacman", Some(&["-S", "nvidia"]))?;

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
                "/etc/mkinitcpio.conf",
                fs::read_to_string("/etc/mkinitcpio.conf")
                    .expect("Error reading from /etc/mkinitcpio.conf")
                    .replace(writing_string[0], writing_string[1]),
            )
            .expect("Error writing to /etc/mkinitcpio.conf");

            run_command("mkinitcpio", Some(&["-p", "linux"]))?;
        }
    }

    // Command set 25
    {
        installation_status.print("Making grub config");

        run_command("grub-mkconfig", Some(&["-o", "/boot/grub/grub.cfg"]))?;
    }

    // Command set 26
    {
        installation_status.print("Enabling network manager service");

        run_command("systemctl", Some(&["enable", "NetworkManager"]))?;
    }

    // Command set 27
    {
        installation_status.print("Installing KDE desktop and applications");

        run_command(
            "pacman",
            Some(&[
                "-Sy",
                "sddm",
                "bludevil",
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
            ]),
        )?;
    }

    // Command set 28
    {
        installation_status.print("Enabling SDDM service");

        run_command("systemctl", Some(&["enable", "sddm"]))?;
    }

    // Command set 29
    {
        installation_status.print("Installing paru aur helper");

        run_command(
            "cd",
            Some(&[format!("/home/{}", app_config.username).as_str()]),
        )?;
        run_command("su", Some(&[format!("{}", app_config.username).as_str()]))?;
        run_command(
            "git",
            Some(&["clone", "https://aur.archlinux.org/paru-bin.git"]),
        )?;
        run_command("makepkg", Some(&["-si"]))?;
    }

    // Printing 'Installation finished' message.
    {
        formatted_print(
            "Installation finished successfully. You can now reboot.",
            PrintFormat::Bordered,
        );
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
            "Error! external process exited with error code: {}",
            exit_code
        )))
    }
}
