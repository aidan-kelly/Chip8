use rand::random;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const START_ADDR: u16 = 0x200;

//Each letter is 5 rows of 8 bits.
//5 bytes per letter, 16 letters, FONTSET_SIZE = 80.
const FONTSET_SIZE: usize = 80;
const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

//Core to emulation/cpu in general
//is fetch-decode-execute loop!
pub struct Emu {
    pc: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_reg: [u8; NUM_REGS],
    i_reg: u16,                                     //Instruction register? 
    sp: u16,                                        //Stack Pointer, points to tippy top of stack.
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    dt: u8,                                         //Delay Timer
    st: u8,                                         //Sound Timer
}

impl Emu {
    pub fn new() -> Self {
        let mut new_emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_reg: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            dt: 0,
            st: 0,
        };

        new_emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        new_emu
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = (START_ADDR as usize) + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }

    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    pub fn tick(&mut self) {
        //Fetch
        let op = self.fetch();

        //Decode & Execute
        self.execute(op);
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;
        let op = (higher_byte << 8) | lower_byte;
        self.pc += 2;
        op
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        if self.st > 0 {
            if self.st == 1 {
                //beep
            }
            self.st -= 1;
        }
    }

    fn execute(&mut self, op: u16) {
        let digit1 = (op & 0xF000) >> 12;
        let digit2 = (op & 0x0F00) >> 8;
        let digit3 = (op & 0x00F0) >> 4;
        let digit4 = op & 0x000F;

        let vx = digit2 as usize;
        let vy = digit3 as usize;
        let n = digit4;
        let nn = (op & 0xFF) as u8;
        let nnn = op & 0xFFF;

        match (digit1, digit2, digit3, digit4) {
            //00E0: Clear screen.
            (0, 0, 0xE, 0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            },
            //00EE: Return from subroutine.
            (0, 0, 0xE, 0xE) => {
                self.pc = self.pop();
            },
            //1NNN: Jump
            (1, _, _, _) => {
                self.pc = nnn;
            },
            //2NNN: Subroutine
            (2, _, _, _) => {
                self.push(self.pc);
                self.pc = nnn;
            },
            //3XNN: Skip if VX == NN
            (3, _, _, _) => {
                if self.v_reg[vx] == nn {
                    self.pc += 2;
                }
            },
            //4XNN: Skip if VX != NN
            (4, _, _, _) => {
                if self.v_reg[vx] != nn {
                    self.pc += 2;
                }
            },
            //5XY0: Skip if VX == VY
            (5, _, _, 0) => {
                if self.v_reg[vx] == self.v_reg[vy] {
                    self.pc += 2;
                }
            },
            //6XNN: Set
            (6, _, _, _) => {
                self.v_reg[vx] = nn;
            },
            //7XNN: Add
            (7, _, _, _) => {
                self.v_reg[vx] = self.v_reg[vx].wrapping_add(nn);
            },
            //8XY0: Set VX = VY
            (8, _, _, 0) => {
                self.v_reg[vx] = self.v_reg[vy];
            },
            //8XY1: Binary OR
            (8, _, _, 1) => {
                self.v_reg[vx] |= self.v_reg[vy];
            },
            //8XY2: Binary AND
            (8, _, _, 2) => {
                self.v_reg[vx] &= self.v_reg[vy];
            },
            //8XY3: Logical XOR
            (8, _, _, 3) => {
                self.v_reg[vx] ^= self.v_reg[vy];
            },
            //8XY4: Add
            (8, _, _, 4) => {
                let (new_vx, carry) = self.v_reg[vx].overflowing_add(self.v_reg[vy]);
                self.v_reg[0xF] = if carry { 1 } else { 0 };
                self.v_reg[vx] = new_vx;
            },
            //8XY5: Subtract
            (8, _, _, 5) => {
                let (new_vx, borrow) = self.v_reg[vx].overflowing_sub(self.v_reg[vy]);
                self.v_reg[0xF] = if borrow { 0 } else { 1 };
                self.v_reg[vx] = new_vx;
            },
            //8XY6: Shift
            (8, _, _, 6) => {
                let lsb = self.v_reg[vx] & 1;
                self.v_reg[vx] >>= 1;
                self.v_reg[0xF] = lsb;
            },
            //8XY7: Subtract
            (8, _, _, 7) => {
                let (new_vx, borrow) = self.v_reg[vy].overflowing_sub(self.v_reg[vx]);
                self.v_reg[0xF] = if borrow { 0 } else { 1 };
                self.v_reg[vx] = new_vx;
            },
            //8XYE: Shift
            (8, _, _, 0xE) => {
                let msb = (self.v_reg[vx] >> 7) & 1;
                self.v_reg[vx] <<= 1;
                self.v_reg[0xF] = msb;
            },
            //9XY0: Ship if VX != VY
            (9, _, _, 0) => {
                if self.v_reg[vx] != self.v_reg[vy] {
                    self.pc += 2;
                }
            },
            //ANNN: Set index
            (0xA, _, _, _) => {
                self.i_reg = nnn;
            },
            //BNNN: Jump with offset
            (0xB, _, _, _) => {
                self.pc = nnn + (self.v_reg[0] as u16);
            },
            //CXNN: Random
            (0xC, _, _, _) => {
                let rng: u8 = random();
                self.v_reg[vx] = rng & nn;
            },
            //DXYN: Display
            (0xD, _, _, _) => {
                let x_coord = self.v_reg[vx] as u16;
                let y_coord = self.v_reg[vy] as u16;
                let mut flipped = false;
                for y_line in 0..n {
                    let sprite_row = self.ram[(self.i_reg + y_line as u16) as usize];

                    for x_line in 0..8{
                        if(sprite_row & (0b1000_0000 >> x_line)) != 0 {
                            let x = (x_coord + x_line) as usize % SCREEN_WIDTH;
                            let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;

                            let index = x + SCREEN_WIDTH * y;

                            flipped |= self.screen[index];
                            self.screen[index] ^= true;
                        }
                    }
                }
                if flipped {
                    self.v_reg[0xF] = 1;
                } else {
                    self.v_reg[0xF] = 0;
                }
            },
            //EX9E: Skip if key
            (0xE, _, 9, 0xE) => {
                if self.keys[self.v_reg[vx] as usize] {
                    self.pc += 2;
                }
            },
            //EXA1: Skip if key
            (0xE, _, 0xA, 1) => {
                if !self.keys[self.v_reg[vx] as usize] {
                    self.pc += 2;
                }
            },
            //FX07: Get delay timer value
            (0xF, _, 0, 7) => {
                self.v_reg[vx] = self.dt;
            },
            //FX15: set delay timer value
            (0xF, _, 1, 5) => {
                self.dt = self.v_reg[vx];
            },
            //FX18: set sound timer value
            (0xF, _, 1, 8) => {
                self.st = self.v_reg[vx];
            },
            //FX1E: add to index
            (0xF, _, 1, 0xE) => {
                self.i_reg = self.i_reg.wrapping_add(self.v_reg[vx] as u16);
            },
            //FX0A: Get key.
            (0xF, _, 0, 0xA) => {
                let mut pressed = false;

                for key_index in 0..NUM_KEYS {
                    if self.keys[key_index] {
                        self.v_reg[vx] = key_index as u8;
                        pressed = true;
                    }
                }
                if !pressed {
                    self.pc -= 2;
                }
            },
            //FX29: Font character.
            (0xF, _, 2, 9) => {
                self.i_reg = self.v_reg[vx] as u16 * 5;
            },
            //FX33: Binary-coded decimal conversion.
            (0xF, _, 3, 3) => {
                let value = self.v_reg[vx] as f32;

                let hundreds = (value / 100.0).floor() as u8;
                let tens = (value / 10.0).floor() as u8;
                let ones = (value / 10.0) as u8;

                self.ram[self.i_reg as usize] = hundreds;
                self.ram[(self.i_reg + 1) as usize] = tens;
                self.ram[(self.i_reg + 2) as usize] = ones;
            },
            //FX55: Store in memory.
            (0xF, _, 5, 5) => {
                
            },
            //FX65: Load memory.
            (0xF, _, 6, 5) => {
                
            },
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", op),
        }
    }

}
