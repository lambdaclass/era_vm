pragma solidity >=0.8.20;

contract Send {
    constructor() {
        address payable self = payable(msg.sender);
        self.call{value: 100};
    }
}
