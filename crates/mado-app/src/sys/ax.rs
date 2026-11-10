//! The Accessibility API and the few CoreFoundation calls around it,
//! declared by hand: the crates that wrap them pull in far more than the
//! calls used here.
//!
//! CoreFoundation strings and arrays are the same objects as Foundation's
//! (toll-free bridged), so a value read here is looked at as an
//! `NSString` or `NSArray` rather than through CF's own getters. CF
//! objects may be used from any thread, and every call here runs off the
//! main thread: each is a question to another app.

use std::ffi::c_void;

use objc2_foundation::{NSNumber, NSString};

pub type Ref = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGSize {
    w: f64,
    h: f64,
}

const VALUE_POINT: u32 = 1;
const VALUE_SIZE: u32 = 2;

/// What the system answers an app that Accessibility has not been given.
pub const API_DISABLED: i32 = -25211;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: Ref) -> bool;
    static kAXTrustedCheckOptionPrompt: Ref;
    fn AXUIElementCreateSystemWide() -> Ref;
    fn AXUIElementCreateApplication(pid: i32) -> Ref;
    fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
    fn AXUIElementSetAttributeValue(element: Ref, attribute: Ref, value: Ref) -> i32;
    fn AXUIElementSetMessagingTimeout(element: Ref, seconds: f32) -> i32;
    fn AXUIElementGetPid(element: Ref, pid: *mut i32) -> i32;
    fn AXUIElementCopyElementAtPosition(element: Ref, x: f32, y: f32, out: *mut Ref) -> i32;
    fn AXUIElementPerformAction(element: Ref, action: Ref) -> i32;
    fn AXValueCreate(kind: u32, value: *const c_void) -> Ref;
    fn AXValueGetValue(value: Ref, kind: u32, out: *mut c_void) -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRetain(r: Ref) -> Ref;
    fn CFRelease(r: Ref);
    fn CFHash(r: Ref) -> usize;
    fn CFDictionaryCreate(alloc: Ref, keys: *const Ref, values: *const Ref, n: isize, key_cb: Ref, value_cb: Ref) -> Ref;
    fn CFBooleanGetValue(b: Ref) -> bool;
    fn CFGetTypeID(r: Ref) -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFArrayGetCount(array: Ref) -> isize;
    fn CFArrayGetValueAtIndex(array: Ref, at: isize) -> Ref;
    fn CFArrayGetTypeID() -> usize;
    fn CFDictionaryGetValue(dict: Ref, key: Ref) -> Ref;
    fn CFNumberGetValue(number: Ref, kind: isize, out: *mut c_void) -> bool;
    static kCFBooleanTrue: Ref;
    static kCFBooleanFalse: Ref;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> Ref;
}

/// `kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements`.
const ON_SCREEN: u32 = (1 << 0) | (1 << 4);
/// `kCFNumberDoubleType`.
const NUMBER_DOUBLE: isize = 13;

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const std::ffi::c_char) -> *mut c_void;
}

/// Whether the system lets Mado read and move other apps' windows. It is
/// asked by doing it: the Dock, which always runs and is never Mado, is
/// asked its role, and a refusal for want of Accessibility says no. Never
/// the app in front: with Mado's settings in front, Mado answered its own
/// question and said yes without the grant. `AXIsProcessTrusted` answers a
/// running process with what it was at launch, so a grant made while Mado
/// runs would go unseen; it decides only when the question says neither.
pub fn trusted() -> bool {
    use objc2_app_kit::NSRunningApplication;
    let dock = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str("com.apple.dock")).iter().map(|a| a.processIdentifier()).find(|&pid| pid > 0);
    let Some(pid) = dock else { return unsafe { AXIsProcessTrusted() } };
    let app = unsafe { AXUIElementCreateApplication(pid) };
    if app.is_null() {
        return unsafe { AXIsProcessTrusted() };
    }
    let attr = NSString::from_str("AXRole");
    let mut out: Ref = std::ptr::null_mut();
    let err = unsafe { AXUIElementCopyAttributeValue(app, as_ref(&attr), &mut out) };
    unsafe {
        if !out.is_null() {
            CFRelease(out);
        }
        CFRelease(app);
    }
    match err {
        0 => true,
        API_DISABLED => false,
        _ => unsafe { AXIsProcessTrusted() },
    }
}

/// The system's own question, which puts Mado in Accessibility's list
/// and offers to open it there.
pub fn ask_trust() {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(std::ptr::null_mut(), keys.as_ptr(), values.as_ptr(), 1, &kCFTypeDictionaryKeyCallBacks as *const c_void as Ref, &kCFTypeDictionaryValueCallBacks as *const c_void as Ref);
        AXIsProcessTrustedWithOptions(options);
        CFRelease(options);
    }
}

/// How long any question to any app may take, set once for the process.
/// A timeout set on one app's element covers that element alone; its
/// windows would wait the system's six seconds on an app that hangs.
pub fn limit_waits() {
    unsafe {
        let all = AXUIElementCreateSystemWide();
        if !all.is_null() {
            AXUIElementSetMessagingTimeout(all, TIMEOUT);
            CFRelease(all);
        }
    }
}

const TIMEOUT: f32 = 0.25;

/// The windows on screen in the Space shown, front to back, as the
/// window server lists them: each one's app and number. Only an app's
/// own windows (layer 0) that can be seen, not the menu bar, the Dock or
/// a panel floating over them. It asks the window server, not the apps,
/// and answers at once.
pub fn windows_on_screen() -> Vec<(i32, u64)> {
    let list = unsafe { CGWindowListCopyWindowInfo(ON_SCREEN, 0) };
    if list.is_null() {
        return Vec::new();
    }
    let list = Value(list);
    let number = |dict: Ref, key: &str| -> Option<f64> {
        let k = NSString::from_str(key);
        let v = unsafe { CFDictionaryGetValue(dict, as_ref(&k)) };
        let mut out = 0f64;
        (!v.is_null() && unsafe { CFNumberGetValue(v, NUMBER_DOUBLE, &mut out as *mut f64 as *mut c_void) }).then_some(out)
    };
    let n = unsafe { CFArrayGetCount(list.0) };
    (0..n)
        .filter_map(|i| {
            let dict = unsafe { CFArrayGetValueAtIndex(list.0, i) };
            let shown = number(dict, "kCGWindowLayer") == Some(0.) && number(dict, "kCGWindowAlpha").is_some_and(|a| a > 0.);
            shown.then(|| Some((number(dict, "kCGWindowOwnerPID")? as i32, number(dict, "kCGWindowNumber")? as u64))).flatten()
        })
        .collect()
}

/// The window the window server shows topmost at a point, in the
/// Accessibility API's points: its app and number. Of an app's own
/// windows (layer 0) only, as `windows_on_screen`. This is the window a
/// press there drags; Accessibility's own hit test asks the apps, and
/// one that does not answer for the point gives the window under it.
pub fn window_under(x: f64, y: f64) -> Option<(i32, u64)> {
    let list = unsafe { CGWindowListCopyWindowInfo(ON_SCREEN, 0) };
    if list.is_null() {
        return None;
    }
    let list = Value(list);
    let number = |dict: Ref, key: &str| -> Option<f64> {
        let k = NSString::from_str(key);
        let v = unsafe { CFDictionaryGetValue(dict, as_ref(&k)) };
        let mut out = 0f64;
        (!v.is_null() && unsafe { CFNumberGetValue(v, NUMBER_DOUBLE, &mut out as *mut f64 as *mut c_void) }).then_some(out)
    };
    let bounds_key = NSString::from_str("kCGWindowBounds");
    let n = unsafe { CFArrayGetCount(list.0) };
    (0..n).find_map(|i| {
        let dict = unsafe { CFArrayGetValueAtIndex(list.0, i) };
        if number(dict, "kCGWindowLayer") != Some(0.) || !number(dict, "kCGWindowAlpha").is_some_and(|a| a > 0.) {
            return None;
        }
        let b = unsafe { CFDictionaryGetValue(dict, as_ref(&bounds_key)) };
        if b.is_null() {
            return None;
        }
        let (bx, by, bw, bh) = (number(b, "X")?, number(b, "Y")?, number(b, "Width")?, number(b, "Height")?);
        (x >= bx && x < bx + bw && y >= by && y < by + bh).then(|| Some((number(dict, "kCGWindowOwnerPID")? as i32, number(dict, "kCGWindowNumber")? as u64))).flatten()
    })
}

/// An owned reference to an Accessibility element: an app or a window.
pub struct Element(Ref);

unsafe impl Send for Element {}

impl Drop for Element {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

/// A value an attribute holds, released when dropped.
struct Value(Ref);

impl Drop for Value {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

fn as_ref(s: &NSString) -> Ref {
    s as *const NSString as Ref
}

/// The window server's number for a window, by the private call every
/// window manager on the Mac uses, looked up by name so a macOS without
/// it loses the number and nothing else.
type GetWindow = unsafe extern "C" fn(Ref, *mut u32) -> i32;

fn get_window() -> Option<GetWindow> {
    static FOUND: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let at = *FOUND.get_or_init(|| unsafe { dlsym(-2isize as *mut c_void, c"_AXUIElementGetWindow".as_ptr()) as usize });
    (at != 0).then(|| unsafe { std::mem::transmute::<usize, GetWindow>(at) })
}

impl Element {
    /// The whole system, through which the app in front is found.
    pub fn system() -> Option<Element> {
        let r = unsafe { AXUIElementCreateSystemWide() };
        (!r.is_null()).then(|| Element(r))
    }

    /// The app with this process id; every question to it gives up after
    /// a quarter of a second.
    pub fn app(pid: i32) -> Option<Element> {
        let r = unsafe { AXUIElementCreateApplication(pid) };
        if r.is_null() {
            return None;
        }
        unsafe { AXUIElementSetMessagingTimeout(r, TIMEOUT) };
        Some(Element(r))
    }

    /// The window under a point of the screen, in the API's points: the
    /// element there, or the window it belongs to.
    pub fn window_at(x: f64, y: f64) -> Option<Element> {
        let system = Element::system()?;
        let mut out: Ref = std::ptr::null_mut();
        let err = unsafe { AXUIElementCopyElementAtPosition(system.0, x as f32, y as f32, &mut out) };
        if err != 0 || out.is_null() {
            return None;
        }
        unsafe { AXUIElementSetMessagingTimeout(out, TIMEOUT) };
        let el = Element(out);
        if el.string("AXRole").as_deref() == Some("AXWindow") {
            return Some(el);
        }
        el.element("AXWindow").ok()
    }

    fn copy(&self, attribute: &str) -> Result<Value, i32> {
        let attr = NSString::from_str(attribute);
        let mut out: Ref = std::ptr::null_mut();
        let err = unsafe { AXUIElementCopyAttributeValue(self.0, as_ref(&attr), &mut out) };
        if err == 0 && !out.is_null() {
            Ok(Value(out))
        } else {
            Err(err)
        }
    }

    /// The element an attribute names (an app's focused window), or the
    /// system's error.
    pub fn element(&self, attribute: &str) -> Result<Element, i32> {
        let v = self.copy(attribute)?;
        let r = v.0;
        std::mem::forget(v);
        unsafe { AXUIElementSetMessagingTimeout(r, TIMEOUT) };
        Ok(Element(r))
    }

    /// An app's windows, the minimized and those of other Spaces among
    /// them when the app lists them.
    pub fn windows(&self) -> Vec<Element> {
        self.list("AXWindows")
    }

    /// The elements under this one: a menu bar's menus, a menu's items.
    pub fn children(&self) -> Vec<Element> {
        self.list("AXChildren")
    }

    fn list(&self, attribute: &str) -> Vec<Element> {
        let Ok(v) = self.copy(attribute) else { return Vec::new() };
        if unsafe { CFGetTypeID(v.0) } != unsafe { CFArrayGetTypeID() } {
            return Vec::new();
        }
        let n = unsafe { CFArrayGetCount(v.0) };
        (0..n)
            .filter_map(|i| {
                let r = unsafe { CFArrayGetValueAtIndex(v.0, i) };
                (!r.is_null()).then(|| {
                    unsafe { AXUIElementSetMessagingTimeout(r, TIMEOUT) };
                    Element(unsafe { CFRetain(r) })
                })
            })
            .collect()
    }

    pub fn string(&self, attribute: &str) -> Option<String> {
        let v = self.copy(attribute).ok()?;
        let obj = unsafe { &*(v.0 as *const objc2::runtime::AnyObject) };
        obj.downcast_ref::<NSString>().map(|s| s.to_string())
    }

    pub fn bool(&self, attribute: &str) -> Option<bool> {
        let v = self.copy(attribute).ok()?;
        unsafe {
            if CFGetTypeID(v.0) == CFBooleanGetTypeID() {
                return Some(CFBooleanGetValue(v.0));
            }
        }
        let obj = unsafe { &*(v.0 as *const objc2::runtime::AnyObject) };
        obj.downcast_ref::<NSNumber>().map(|n| n.boolValue())
    }

    pub fn number(&self, attribute: &str) -> Option<f64> {
        let v = self.copy(attribute).ok()?;
        let obj = unsafe { &*(v.0 as *const objc2::runtime::AnyObject) };
        obj.downcast_ref::<NSNumber>().map(|n| n.doubleValue())
    }

    /// The element pressed, as a click on it: a menu's item chosen.
    pub fn press(&self) -> bool {
        let action = NSString::from_str("AXPress");
        unsafe { AXUIElementPerformAction(self.0, as_ref(&action)) == 0 }
    }

    pub fn set_bool(&self, attribute: &str, value: bool) -> bool {
        let attr = NSString::from_str(attribute);
        let v = unsafe { if value { kCFBooleanTrue } else { kCFBooleanFalse } };
        unsafe { AXUIElementSetAttributeValue(self.0, as_ref(&attr), v) == 0 }
    }

    /// Where the window's top left is, in the API's points.
    pub fn position(&self) -> Option<(f64, f64)> {
        let v = self.copy("AXPosition").ok()?;
        let mut p = CGPoint::default();
        unsafe { AXValueGetValue(v.0, VALUE_POINT, &mut p as *mut CGPoint as *mut c_void) }.then_some((p.x, p.y))
    }

    pub fn size(&self) -> Option<(f64, f64)> {
        let v = self.copy("AXSize").ok()?;
        let mut s = CGSize::default();
        unsafe { AXValueGetValue(v.0, VALUE_SIZE, &mut s as *mut CGSize as *mut c_void) }.then_some((s.w, s.h))
    }

    pub fn set_position(&self, x: f64, y: f64) -> bool {
        let p = CGPoint { x, y };
        self.set_value("AXPosition", unsafe { AXValueCreate(VALUE_POINT, &p as *const CGPoint as *const c_void) })
    }

    pub fn set_size(&self, w: f64, h: f64) -> bool {
        let s = CGSize { w, h };
        self.set_value("AXSize", unsafe { AXValueCreate(VALUE_SIZE, &s as *const CGSize as *const c_void) })
    }

    fn set_value(&self, attribute: &str, value: Ref) -> bool {
        if value.is_null() {
            return false;
        }
        let attr = NSString::from_str(attribute);
        let ok = unsafe { AXUIElementSetAttributeValue(self.0, as_ref(&attr), value) == 0 };
        unsafe { CFRelease(value) };
        ok
    }

    pub fn pid(&self) -> Option<i32> {
        let mut pid = 0;
        (unsafe { AXUIElementGetPid(self.0, &mut pid) } == 0).then_some(pid)
    }

    /// What Mado knows a window by between presses: the window server's
    /// number, else the element's own hash with its app's process id.
    pub fn window_id(&self) -> u64 {
        if let Some(f) = get_window() {
            let mut id = 0u32;
            if unsafe { f(self.0, &mut id) } == 0 && id != 0 {
                return id as u64;
            }
        }
        let pid = self.pid().unwrap_or(0) as u64;
        (pid << 32) | (unsafe { CFHash(self.0) } as u64 & 0xFFFF_FFFF) | (1 << 63)
    }
}

impl Clone for Element {
    fn clone(&self) -> Self {
        Element(unsafe { CFRetain(self.0) })
    }
}
