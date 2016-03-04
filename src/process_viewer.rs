#![crate_type = "bin"]

extern crate gtk;
extern crate glib;
extern crate sysinfo;

use gtk::{Orientation, SortType, Widget};
use gtk::prelude::*;

use sysinfo::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

struct NoteBook {
    notebook: gtk::Notebook,
    tabs: Vec<gtk::Box>,
}

impl NoteBook {
    fn new() -> NoteBook {
        NoteBook {
            notebook: gtk::Notebook::new(),
            tabs: Vec::new(),
        }
    }

    fn create_tab<'a>(&mut self, title: &'a str, widget: &Widget) -> Option<u32> {
        let label = gtk::Label::new(Some(title));
        let tab = gtk::Box::new(Orientation::Horizontal, 0);

        tab.pack_start(&label, false, false, 0);
        tab.show_all();

        let index = self.notebook.append_page(widget, Some(&tab));

        self.tabs.push(tab);

        Some(index)
    }
}

#[allow(dead_code)]
struct Procs {
    left_tree: gtk::TreeView,
    scroll: gtk::ScrolledWindow,
    current_pid: Rc<RefCell<Option<i64>>>,
    kill_button: Rc<RefCell<gtk::Button>>,
    vertical_layout: gtk::Box,
    list_store: gtk::ListStore,
}

unsafe impl Send for Procs {}

impl Procs {
    pub fn new<'a>(proc_list: &HashMap<usize, Process>, note: &mut NoteBook) -> Procs {
        let left_tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        let current_pid = Rc::new(RefCell::new(None));
        let kill_button = Rc::new(RefCell::new(gtk::Button::new_with_label("End task")));
        let current_pid1 = current_pid.clone();
        let current_pid2 = current_pid.clone();
        let kill_button1 = kill_button.clone();

        scroll.set_min_content_height(800);
        scroll.set_min_content_width(600);

        let mut columns : Vec<gtk::TreeViewColumn> = Vec::new();

        append_column("pid", &mut columns);
        append_column("process name", &mut columns);
        append_column("cpu usage", &mut columns);
        append_column("memory usage (in kB)", &mut columns);

        for i in columns.iter() {
            left_tree.append_column(&i);
            i.connect_clicked(|model| {
                //let model = model.downcast::<gtk::TreeSortable>().unwrap();
                model.set_sort_column_id(0);
            });
        }

        let mut list_store = gtk::ListStore::new(&[glib::Type::I64, glib::Type::String,
                                                   glib::Type::String, glib::Type::U32]);
        for (_, pro) in proc_list {
            create_and_fill_model(&mut list_store, pro.pid, &pro.cmd, &pro.name, pro.cpu_usage,
                                  pro.memory);
        }

        left_tree.set_model(Some(&list_store));
        left_tree.set_headers_visible(true);
        scroll.add(&left_tree);
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

        left_tree.connect_cursor_changed(move |tree_view| {
            match tree_view.get_selection() {
                Some(selection) => {
                    if let Some((model, iter)) = selection.get_selected() {
                        let pid = Some(model.get_value(&iter, 0).get().unwrap());
                        let mut tmp = current_pid1.borrow_mut();
                        *tmp = pid;
                    }
                    (*kill_button1.borrow_mut()).set_sensitive((*current_pid.borrow()).is_some());
                }
                None => {
                    let mut tmp = current_pid1.borrow_mut();
                    *tmp = None;
                }
            }
        });
        (*kill_button.borrow_mut()).set_sensitive(false);

        vertical_layout.add(&scroll);
        vertical_layout.add(&(*kill_button.borrow_mut()));
        let vertical_layout : Widget = vertical_layout.upcast();

        note.create_tab("Process list", &vertical_layout);
        Procs {
            left_tree: left_tree,
            scroll: scroll,
            current_pid: current_pid2.clone(),
            kill_button: kill_button,
            vertical_layout: vertical_layout.downcast::<gtk::Box>().unwrap(),
            list_store: list_store,
        }
    }
}

fn append_column(title: &str, v: &mut Vec<gtk::TreeViewColumn>) {
    let l = v.len();
    let renderer = gtk::CellRendererText::new();

    v.push(gtk::TreeViewColumn::new());
    let tmp = v.get_mut(l).unwrap();

    tmp.set_title(title);
    tmp.set_resizable(true);
    tmp.pack_start(&renderer, true);
    tmp.add_attribute(&renderer, "text", l as i32);
    tmp.set_clickable(true);
}

fn create_and_fill_model(list_store: &mut gtk::ListStore, pid: i64, cmdline: &str, name: &str,
                         cpu: f32, memory: u64) {
    if cmdline.len() < 1 {
        return;
    }

    let val1 = pid.to_value();
    let val2 = memory.to_value();
    let top_level = list_store.append();
    list_store.set_value(&top_level, 0, &val1);
    list_store.set_value(&top_level, 1, &name.to_value());
    list_store.set_value(&top_level, 2, &format!("{:.1}", cpu).to_value());
    list_store.set_value(&top_level, 3, &val2);
}

fn update_window(list: &mut gtk::ListStore, system: Arc<Mutex<sysinfo::System>>,
                 info: Arc<Mutex<DisplaySysInfo>>) {
    let system = &mut system.lock().unwrap();
    let info = &mut info.lock().unwrap();

    system.refresh_all();
    let mut entries : HashMap<usize, Process> = system.get_process_list().clone();
    let mut nb = list.iter_n_children(None);

    info.update_ram_display(&system);
    info.update_process_display(&system);

    let mut i = 0;
    while i < nb {
        if let Some(mut iter) = list.iter_nth_child(None, i) {
            let pid : Option<i64> = list.get_value(&iter, 0).get();
            if pid.is_none() {
                i += 1;
                continue;
            }
            let pid = pid.unwrap();
            let mut to_delete = false;

            match entries.get(&(pid as usize)) {
                Some(p) => {
                    let val2 = p.memory.to_value();
                    list.set_value(&iter, 2, &format!("{:.1}", p.cpu_usage).to_value());
                    list.set_value(&iter, 3, &val2);
                    to_delete = true;
                }
                None => {
                    list.remove(&mut iter);
                }
            }
            if to_delete {
                entries.remove(&(pid as usize));
                nb = list.iter_n_children(None);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    for (_, pro) in entries {
        create_and_fill_model(list, pro.pid, &pro.cmd, &pro.name, pro.cpu_usage, pro.memory);
    }
}

#[allow(dead_code)]
struct DisplaySysInfo {
    procs : Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram : Rc<RefCell<gtk::ProgressBar>>,
    swap : Rc<RefCell<gtk::ProgressBar>>,
    vertical_layout : Rc<RefCell<gtk::Box>>,
}

impl DisplaySysInfo {
    pub fn new(sys1: Arc<Mutex<sysinfo::System>>, note: &mut NoteBook) -> DisplaySysInfo {
        let sys2 = sys1.clone();

        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let ram = gtk::ProgressBar::new();
        let swap = gtk::ProgressBar::new();

        ram.set_show_text(true);
        swap.set_show_text(true);
        vertical_layout.set_spacing(5);

        let mut i = 0;
        let mut total = false;

        vertical_layout.pack_start(&gtk::Label::new(Some("Memory usage")), false, false, 15);
        vertical_layout.add(&ram);
        vertical_layout.pack_start(&gtk::Label::new(Some("Swap usage")), false, false, 15);
        vertical_layout.add(&swap);
        vertical_layout.pack_start(&gtk::Label::new(Some("Total CPU usage")), false, false, 15);
        for pro in sys1.lock().unwrap().get_processor_list() {
            if total {
                procs.push(gtk::ProgressBar::new());
                let p : &gtk::ProgressBar = &procs[i];
                let l = gtk::Label::new(Some(&format!("{}", i)));
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);

                p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);
                horizontal_layout.pack_start(&l, false, false, 5);
                horizontal_layout.pack_start(p, true, true, 5);
                vertical_layout.add(&horizontal_layout);
            } else {
                procs.push(gtk::ProgressBar::new());
                let p : &gtk::ProgressBar = &procs[i];

                p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);

                vertical_layout.add(p);
                vertical_layout.pack_start(&gtk::Label::new(Some("Process usage")), false,
                                           false, 15);
                total = true;
            }
            i += 1;
        }

        let vertical_layout : Widget = vertical_layout.upcast();
        note.create_tab("System usage", &vertical_layout);
        let vertical_layout : gtk::Box = vertical_layout.downcast::<gtk::Box>().unwrap();

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram: Rc::new(RefCell::new(ram)),
            swap: Rc::new(RefCell::new(swap)),
            vertical_layout: Rc::new(RefCell::new(vertical_layout)),
        };
        tmp.update_ram_display(&sys2.lock().unwrap());
        tmp
    }

    pub fn update_ram_display(&mut self, sys: &sysinfo::System) {
        let total = sys.get_total_memory();
        let used = sys.get_used_memory();
        let disp = if total < 100000 {
            format!("{} / {}KB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1000, total / 1000)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1000000, total / 1000000)
        } else {
            format!("{} / {}TB", used / 1000000000, total / 1000000000)
        };

        (*self.ram.borrow_mut()).set_text(Some(&disp));
        (*self.ram.borrow_mut()).set_fraction(used as f64 / total as f64);

        let total = sys.get_total_swap();
        let used = total - sys.get_used_swap();
        let disp = if total < 100000 {
            format!("{} / {}KB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1000, total / 1000)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1000000, total / 1000000)
        } else {
            format!("{} / {}TB", used / 1000000000, total / 1000000000)
        };

        (*self.swap.borrow_mut()).set_text(Some(&disp));
        (*self.swap.borrow_mut()).set_fraction(used as f64 / total as f64);
    }

    pub fn update_process_display(&mut self, sys: &sysinfo::System) {
        let v = &*self.procs.borrow_mut();
        let mut i = 0;

        for pro in sys.get_processor_list() {
            v[i].set_text(Some(&format!("{:.1} %", pro.get_cpu_usage() * 100.)));
            v[i].set_show_text(true);
            v[i].set_fraction(pro.get_cpu_usage() as f64);
            i += 1;
        }
    }
}

fn main() {
    gtk::init().expect("GTK couldn't start normally");

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    let sys : Arc<Mutex<sysinfo::System>> = Arc::new(Mutex::new(sysinfo::System::new()));
    let mut note = NoteBook::new();
    let mut procs = Procs::new(sys.lock().unwrap().get_process_list(), &mut note);
    let current_pid2 = procs.current_pid.clone();
    let sys1 = sys.clone();
    let sys2 = sys.clone();

    window.set_title("Process viewer");
    window.set_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });

    { sys.lock().unwrap().refresh_all(); }
    (*procs.kill_button.borrow_mut()).connect_clicked(move |_| {
        let tmp = (*current_pid2.borrow_mut()).is_some() ;

        if tmp {
            let s = (*current_pid2.borrow()).clone();
            match sys.lock().unwrap().get_process(s.unwrap()) {
                Some(p) => {
                    p.kill(Signal::Kill);
                },
                None => {}
            };
        }
    });

    let display_tab = DisplaySysInfo::new(sys2, &mut note);
    let m_display_tab = Arc::new(Mutex::new(display_tab));

    gtk::timeout_add(1500, move || {
        update_window(&mut procs.list_store, sys1.clone(), m_display_tab.clone());
        glib::Continue(true)
    });

    window.add(&note.notebook);
    window.show_all();
    gtk::main();
}
