// Matrix - Kod Yağmuru: GNOME 42–44 için GNOME Shell eklentisi.
//
// GNOME 45 öncesinin eklenti biçimiyle (imports) yazılmış kopyadır. Davranışı
// ../extension.js ile aynıdır; biri değişince öteki de değiştirilir. Ubuntu
// 22.04 (GNOME 42) ve Debian 12 (GNOME 43) içindir; Wayland ve X11
// oturumlarında çalışır.
'use strict';

const {Gio, GLib, Meta} = imports.gi;

const Config = imports.misc.config;
const ExtensionUtils = imports.misc.extensionUtils;
const {InhibitFlags} = imports.misc.gnomeSession;
const Main = imports.ui.main;

const Me = ExtensionUtils.getCurrentExtension();

/** Programın kapatılırken aldığı sinyal (SIGTERM). */
const SIGTERM = 15;

/** GNOME Shell'in ana sürümü. `Meta.WaylandClient.new` GNOME 44'te
 * oturum bağlamını (`global.context`) da almaya başladı. */
const SHELL_MAJOR = parseInt(Config.PACKAGE_VERSION.split('.')[0]);

/**
 * Programı başlatır. Dönen nesne programın sürecini ve pencerelerini tanıma,
 * pencere listesinden gizleme işlevlerini taşır.
 *
 * - Wayland: Program, GNOME'un açtığı özel bir bağlantıyla başlar; böylece
 *   pencereleri kesin olarak tanınır.
 * - X11: Program sıradan bir alt süreç olarak başlar; pencereleri süreç
 *   numarasıyla tanınır. X11'de pencere listesinden gizleme yoktur.
 */
function launch(argv) {
    const launcher = new Gio.SubprocessLauncher({flags: Gio.SubprocessFlags.NONE});
    if (!Meta.is_wayland_compositor()) {
        const subprocess = launcher.spawnv(argv);
        // Süreç numarası bilinmeyen pencereler 0 döndürür; onlar sahiplenilmez.
        const pid = Number(subprocess.get_identifier());
        return {
            subprocess,
            owns: window => pid > 0 && window.get_pid() === pid,
            hide: () => {},
        };
    }
    const client = SHELL_MAJOR >= 44
        ? Meta.WaylandClient.new(global.context, launcher)
        : Meta.WaylandClient.new(launcher);
    return {
        subprocess: client.spawnv(global.display, argv),
        owns: window => client.owns_window(window),
        hide: window => client.hide_from_window_list(window),
    };
}

class MatrixExtension {
    enable() {
        this._settings = ExtensionUtils.getSettings();
        this._idleMonitor = global.backend.get_core_idle_monitor();
        this._idleWatch = 0;
        // Çalışan program (bkz. launch); program yoksa boştur.
        this._program = null;

        this._watchIdle();
        this._settings.connectObject('changed::start-delay', () => this._watchIdle(), this);
        global.display.connectObject('window-created', (_display, window) => this._adopt(window), this);
        // GNOME ekranı karartmaya veya kilitlemeye başladı: çizim durur.
        Main.screenShield.connectObject('active-changed', () => this._stop(), this);
    }

    disable() {
        this._stop();
        this._idleMonitor.remove_watch(this._idleWatch);
        this._settings.disconnectObject(this);
        global.display.disconnectObject(this);
        Main.screenShield.disconnectObject(this);
        this._settings = null;
        this._idleMonitor = null;
    }

    /** Boşta kalma süresini ayardan okur ve bekleyişi yeniden kurar. */
    _watchIdle() {
        if (this._idleWatch)
            this._idleMonitor.remove_watch(this._idleWatch);
        const minutes = this._settings.get_uint('start-delay');
        this._idleWatch = this._idleMonitor.add_idle_watch(minutes * 60 * 1000, () => this._start());
    }

    /** Ekran koruyucuyu başlatır. Video izlenirken olduğu gibi bir uygulama
     * boşta kalmayı engellediyse veya ekran zaten kilitliyse başlamaz. */
    async _start() {
        if (this._program || Main.screenShield.active || Main.sessionMode.isLocked)
            return;
        // Beklerken eklenti kapatılmış veya program başlamış olabilir.
        if (await this._idleInhibited() || this._program || !this._settings)
            return;
        try {
            const program = launch([GLib.build_filenamev([Me.path, 'matrixcoderain']), '/s']);
            this._program = program;
            program.subprocess.wait_async(null, () => {
                if (this._program === program)
                    this._program = null;
            });
        } catch (e) {
            console.error(`Matrix - Kod Yağmuru başlatılamadı: ${e.message}`);
        }
    }

    /** Bir uygulama boşta kalmayı engelliyor mu? Oturum yöneticisine
     * ulaşılamazsa engel yok sayılır; ekran koruyucu yine çalışır. */
    async _idleInhibited() {
        try {
            const reply = await Gio.DBus.session.call(
                'org.gnome.SessionManager', '/org/gnome/SessionManager',
                'org.gnome.SessionManager', 'IsInhibited',
                new GLib.Variant('(u)', [InhibitFlags.IDLE]), new GLib.VariantType('(b)'),
                Gio.DBusCallFlags.NONE, -1, null);
            return reply.deepUnpack()[0];
        } catch (e) {
            console.warn(`Matrix - Kod Yağmuru: boşta kalma engeli okunamadı: ${e.message}`);
            return false;
        }
    }

    /** Programın açtığı pencereyi pencere listesinden gizler, en üste alır ve
     * etkinleştirir; böylece ilk tuş ve fare hareketi programa gider. */
    _adopt(window) {
        if (!this._program?.owns(window))
            return;
        this._program.hide(window);
        window.make_above();
        const shownId = window.connect('shown', () => {
            window.disconnect(shownId);
            Main.activateWindow(window);
        });
    }

    /** Çalışan programı kapatır. */
    _stop() {
        this._program?.subprocess.send_signal(SIGTERM);
        this._program = null;
    }
}

/** GNOME eklentiyi yüklerken bu işlevi çağırır. */
function init() {
    return new MatrixExtension();
}
