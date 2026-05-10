import { useState, useMemo } from 'react'
import { OnlineStatus } from '../types/message'
import { UserOnlineStatus } from './UserOnlineStatus'
import './OnlineUsersList.css'

/**
 * 用户在线状态信息
 */
export interface UserOnlineInfo {
  userId: string
  nickname: string
  avatar?: string
  status: OnlineStatus
}

/**
 * 在线用户列表组件属性
 */
interface OnlineUsersListProps {
  /** 用户列表 */
  users: UserOnlineInfo[]
  /** 是否显示搜索框 */
  showSearch?: boolean
  /** 是否显示状态文字 */
  showStatusLabel?: boolean
  /** 点击用户回调 */
  onUserClick?: (userId: string) => void
  /** 是否显示分组 */
  showGroups?: boolean
  /** 最大高度（像素） */
  maxHeight?: number
}

/**
 * 在线用户列表组件
 * 显示所有在线用户，支持搜索、分组和滚动
 */
export function OnlineUsersList({
  users,
  showSearch = true,
  showStatusLabel = false,
  onUserClick,
  showGroups = true,
  maxHeight = 400,
}: OnlineUsersListProps) {
  const [searchQuery, setSearchQuery] = useState('')

  // 过滤用户
  const filteredUsers = useMemo(() => {
    if (!searchQuery.trim()) {
      return users
    }
    const query = searchQuery.toLowerCase()
    return users.filter(
      (user) =>
        user.nickname.toLowerCase().includes(query) ||
        user.userId.toLowerCase().includes(query)
    )
  }, [users, searchQuery])

  // 按状态分组
  const groupedUsers = useMemo(() => {
    if (!showGroups) {
      return { all: filteredUsers }
    }

    return {
      all: filteredUsers,
      online: filteredUsers.filter((u) => u.status === OnlineStatus.ONLINE) ?? [],
      busy: filteredUsers.filter((u) => u.status === OnlineStatus.BUSY) ?? [],
      away: filteredUsers.filter((u) => u.status === OnlineStatus.AWAY) ?? [],
      offline: filteredUsers.filter((u) => u.status === OnlineStatus.OFFLINE) ?? [],
    }
  }, [filteredUsers, showGroups])

  // 渲染用户列表
  const renderUserList = (userList: UserOnlineInfo[]) => {
    if (userList.length === 0) {
      return (
        <div className="online-users-empty">
          暂无用户
        </div>
      )
    }

    return userList.map((user) => (
      <div key={user.userId} className="online-users-item">
        <UserOnlineStatus
          userId={user.userId}
          avatar={user.avatar}
          nickname={user.nickname}
          status={user.status}
          avatarSize={36}
          clickable={!!onUserClick}
          onClick={onUserClick}
          showStatusLabel={showStatusLabel}
        />
      </div>
    ))
  }

  return (
    <div className="online-users-list">
      {/* 标题和搜索 */}
      <div className="online-users-header">
        <h3 className="online-users-title">
          在线用户
          <span className="online-users-count">({filteredUsers.length})</span>
        </h3>
        {showSearch && (
          <input
            type="text"
            className="online-users-search"
            placeholder="搜索用户..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        )}
      </div>

      {/* 用户列表 */}
      <div className="online-users-body" style={{ maxHeight: `${maxHeight}px` }}>
        {showGroups ? (
          <>
            {/* 在线 */}
            {groupedUsers.online && groupedUsers.online.length > 0 && (
              <div className="online-users-group">
                <div className="online-users-group-header">
                  <span className="group-name">在线</span>
                  <span className="group-count">({groupedUsers.online.length})</span>
                </div>
                <div className="online-users-group-body">
                  {renderUserList(groupedUsers.online)}
                </div>
              </div>
            )}

            {/* 忙碌 */}
            {groupedUsers.busy && groupedUsers.busy.length > 0 && (
              <div className="online-users-group">
                <div className="online-users-group-header">
                  <span className="group-name">忙碌</span>
                  <span className="group-count">({groupedUsers.busy.length})</span>
                </div>
                <div className="online-users-group-body">
                  {renderUserList(groupedUsers.busy)}
                </div>
              </div>
            )}

            {/* 离开 */}
            {groupedUsers.away && groupedUsers.away.length > 0 && (
              <div className="online-users-group">
                <div className="online-users-group-header">
                  <span className="group-name">离开</span>
                  <span className="group-count">({groupedUsers.away.length})</span>
                </div>
                <div className="online-users-group-body">
                  {renderUserList(groupedUsers.away)}
                </div>
              </div>
            )}

            {/* 离线 */}
            {groupedUsers.offline && groupedUsers.offline.length > 0 && (
              <div className="online-users-group">
                <div className="online-users-group-header">
                  <span className="group-name">离线</span>
                  <span className="group-count">({groupedUsers.offline.length})</span>
                </div>
                <div className="online-users-group-body">
                  {renderUserList(groupedUsers.offline)}
                </div>
              </div>
            )}

            {/* 空状态 */}
            {filteredUsers.length === 0 && (
              <div className="online-users-empty">
                {searchQuery ? '未找到匹配的用户' : '暂无用户'}
              </div>
            )}
          </>
        ) : (
          <div className="online-users-list-body">
            {renderUserList(filteredUsers)}
          </div>
        )}
      </div>
    </div>
  )
}
