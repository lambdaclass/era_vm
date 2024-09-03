pragma solidity >=0.8.20;

contract Send {
    constructor() payable {
        address payable other = payable(0x888888CfAebbEd5554c3F36BfBD233f822e9455f);
        uint256 success;

        assembly {
          success := call(gas(), other, 100, 0, 0, 0, 0)
        }

        require(success != 0, "Failed");
    }
}
