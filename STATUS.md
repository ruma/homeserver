# Ruma Status

The following chart shows the status of support for individual client-server API endpoints.
If you're interested in working on an API, take a look at the corresponding tracking issue and leave a comment indicating your interest!

Legend:

:white_check_mark: Supported :construction: Partialially supported :no_entry_sign: Not supported

<table>
  <tr>
    <th>Status</th>
    <th>Issue</th>
    <th>API endpoint</th>
  </tr>
  <tr>
    <th align="left" colspan="3">Versions</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>GET /versions</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Login</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /login</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td></td>
    <td>POST /tokenrefresh</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /logout</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Account registration and management</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /register</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /account/password</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Adding account administrative contact information</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#4</td>
    <td>POST /account/3pid</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#5</td>
    <td>GET /account/3pid</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Filtering</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#6</td>
    <td>GET /user/:user_id/filter/:filter_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#7</td>
    <td>POST /user/:user_id/filter</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Syncing events</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#8</td>
    <td>GET /sync</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Getting events for a room</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#9</td>
    <td>GET /rooms/:room_id/state</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#10</td>
    <td>GET /rooms/:room_id/members</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#11</td>
    <td>GET /rooms/:room_id/state/:event_type/:state_key</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#12</td>
    <td>GET /rooms/:room_id/state/:event_type</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#13</td>
    <td>GET /rooms/:room_id/messages</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Sending events to a room</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#14</td>
    <td>PUT /rooms/:room_id/state/:event_type</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#15</td>
    <td>PUT /rooms/:room_id/state/:event_type/:state_key</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#16</td>
    <td>PUT /rooms/:room_id/send/:event_type/:transaction_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Redactions</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#17</td>
    <td>PUT /rooms/:room_id/redact/:event_id/:transaction_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Room creation</th>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td>#18</td>
    <td>POST /createRoom</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Room aliases</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#19</td>
    <td>PUT /directory/room/:room_alias</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#20</td>
    <td>DELETE /directory/room/:room_alias</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#21</td>
    <td>GET /directory/room/:room_alias</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Joining rooms</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#22</td>
    <td>POST /rooms/:room_id/invite</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#23</td>
    <td>POST /join/:room_id_or_alias</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#24</td>
    <td>POST /rooms/:room_id/join</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#25</td>
    <td>POST /rooms/:room_id/kick</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#26</td>
    <td>POST /rooms/:room_id/unban</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#27</td>
    <td>POST /rooms/:room_id/ban</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Leaving rooms</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#28</td>
    <td>POST /rooms/:room_id/forget</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#29</td>
    <td>POST /rooms/:room_id/leave</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Listing rooms</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#30</td>
    <td>GET /publicRooms</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Profiles</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#31</td>
    <td>PUT /profile/:user_id/displayname</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#32</td>
    <td>GET /profile/:user_id/displayname</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#33</td>
    <td>PUT /profile/:user_id/avatar_url</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#34</td>
    <td>GET /profile/:user_id/avatar_url</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#35</td>
    <td>GET /profile/:user_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Voice over IP</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#36</td>
    <td>GET /voip/turnServer</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Typing notifications</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#37</td>
    <td>PUT /rooms/:room_id/typing/:user_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Receipts</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#38</td>
    <td>POST /rooms/:room_id/receipt/:receipt_type/:event_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Presence</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#39</td>
    <td>PUT /presence/:user_id/status</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#40</td>
    <td>GET /presence/:user_id/status</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#41</td>
    <td>POST /presence/list/:user_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#42</td>
    <td>GET /presence/list/:user_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Content repository</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#43</td>
    <td>GET /download/:server_name/:media_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#44</td>
    <td>POST /upload</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#45</td>
    <td>GET /thumbnail/:server_name/:media_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Push notifications</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#46</td>
    <td>POST /pushers/set</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#47</td>
    <td>GET /pushers</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Push notification rules</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#48</td>
    <td>PUT /pushrules/:scope/:kind/:rule_id/enabled</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#49</td>
    <td>GET /pushrules/:scope/:kind/:rule_id/enabled</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#50</td>
    <td>PUT /pushrules/:scope/:kind/:rule_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#51</td>
    <td>DELETE /pushrules/:scope/:kind/:rule_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#52</td>
    <td>GET /pushrules/:scope/:kind/:rule_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#53</td>
    <td>GET /pushrules</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#54</td>
    <td>PUT /pushrules/:scope/:kind/:rule_id/actions</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#55</td>
    <td>GET /pushrules/:scope/:kind/:rule_id/actions</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Third party invites</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#56</td>
    <td>POST /rooms/:room_id/invite</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Server side search</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#57</td>
    <td>POST /search</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Room previews</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#58</td>
    <td>GET /events</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Room tagging</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#59</td>
    <td>PUT /user/:user_id/rooms/:room_id/tags/:tag</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#60</td>
    <td>DELETE /user/:user_id/rooms/:room_id/tags/:tag</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#61</td>
    <td>GET /user/:user_id/rooms/:room_id/tags</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Client config</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#62</td>
    <td>PUT /user/:user_id/rooms/:room_id/account_data/:type</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#63</td>
    <td>PUT /user/:user_id/account_data/:type</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Server administration</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#64</td>
    <td>GET /admin/whois/:user_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Event context</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td>#65</td>
    <td>GET /rooms/:room_id/context/:event_id</td>
  </tr>
</table>
